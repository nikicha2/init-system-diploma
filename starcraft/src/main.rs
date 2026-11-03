
use std::io::{self};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::ptr;
use std::mem::{size_of, zeroed};

use signal_hook::consts::SIGCHLD;
use signal_hook::iterator::{Handle as SignalsHandle, Signals};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use libc::{self};
use std::ffi::CString;

mod ipc;

const INITIAL_SCVS: usize = 5;
const TARGET_MARINES: i32 = 20;
const MAX_UNITS: i32 = 200;
const BLOCK_MINERALS: i32 = 500;
const UNIT_COST: i32 = 50;
const MAX_BLOCKS: usize = 128;

struct GameState {
    map_minerals: Vec<i32>,
    player_minerals: i32,
    scv_count: i32,
    marine_count: i32,
}

#[repr(C)]
struct SharedState {
    map_blocks: i32,
    map_minerals: [i32; MAX_BLOCKS],

    player_minerals: i32,
    scv_count: i32,
    marine_count: i32,
    units_training: i32,
    next_scv_id: i32,

    // Set to 1 to request a graceful shutdown.
    shutdown: i32,

    // Protects the shared game counters.
    state_sem: libc::sem_t,

    // Allows only one SCV to unload at the Command Center.
    cc_sem: libc::sem_t,

    // One semaphore per mineral block.
    block_sems: [libc::sem_t; MAX_BLOCKS],
}

struct SharedGame {
    ptr: *mut SharedState,
    name: CString,
    size: usize,
}


#[derive(Debug)]
enum ChildKind {
    Worker { id: i32 },
    MarineTraining,
    ScvTraining { id: i32 },
}

struct ChildInfo {
    kind: ChildKind,
}

type Children = HashMap<libc::pid_t, ChildInfo>;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let map_blocks: usize = args
        .get(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(2);

    if map_blocks == 0 {
        eprintln!("The map must have at least one mineral block.");
        return Ok(());
    }

    let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    let (signal_handle, reaper_thread) =
        start_child_reaper(tx.clone())?;

    let input_thread = start_input_reader(tx.clone());

    let mut game = GameState {
        map_minerals: vec![BLOCK_MINERALS; map_blocks],
        player_minerals: 0,
        scv_count: INITIAL_SCVS as i32,
        marine_count: 0,
    };

    let mut shared = create_shared_game(&game, map_blocks)?;

    let mut children: Children = HashMap::new();

    for id in 1..=INITIAL_SCVS {
        let id = id as i32;
        let child = spawn_worker(&shared, id)?;

        register_child(
            &mut children,
            child,
            ChildKind::Worker { id },
        );
    }

    let socket_server = ipc::start_server(tx.clone())?;
    let mut should_exit = false;
    loop {
        match rx.recv() {
            Ok(Event::Input(command)) => {
                match command.trim() {
                    "m" => {
                        if reserve_unit(&mut shared, UNIT_COST, MAX_UNITS)? {
                            match spawn_marine_training() {
                                Ok(child) => {
                                    register_child(
                                        &mut children,
                                        child,
                                        ChildKind::MarineTraining,
                                    );
                                }
                                Err(error) => {
                                    finish_training(
                                        &shared,
                                        ChildKind::MarineTraining,
                                        false,
                                    )?;

                                    return Err(error);
                                }
                            }
                        } else {
                            println!("Not enough minerals or unit capacity.");
                        }
                    }

                    "s" => {
                        if reserve_unit(&mut shared, UNIT_COST, MAX_UNITS)? {
                            let id = reserve_next_scv_id(&mut shared)?;

                            match spawn_scv_training() {
                                Ok(child) => {
                                    register_child(
                                        &mut children,
                                        child,
                                        ChildKind::ScvTraining { id },
                                    );
                                }
                                Err(error) => {
                                    finish_training(
                                        &shared,
                                        ChildKind::ScvTraining { id },
                                        false,
                                    )?;

                                    return Err(error);
                                }
                            }
                        } else {
                            println!("Not enough minerals or too many units.");
                        }
                    }

                    "" => {}

                    _ => println!("Unknown command."),
                }
            }

            Ok(Event::InputClosed) => {
                println!("Input closed.");
                break;
            }

            Ok(Event::InputError(error)) => {
                eprintln!("Reading stdin failed: {error}");
                break;
            }

            Ok(Event::ChildExited { pid, status }) => {
                handle_child_exit(pid, status, &mut children, &shared)?;
            }

            Ok(Event::ReaperError(error)) => {
                return Err(io::Error::other(
                    format!("Child reaper failed: {error}"),
                ));
            }

            Ok(Event::Socket(request)) => {
                should_exit = handle_socket_request(request, &shared)?;
            }

            Err(_) => {
                // Every Sender has been dropped.
                break;
            }
        }
        
        if should_exit {
            break;
        }

        game = read_shared_game(&shared)?;

        if game.marine_count >= TARGET_MARINES {
            println!("Target number of marines reached. You win!");
            break;
        }

        if game.map_minerals.iter().all(|&m| m == 0) {
            println!("All minerals have been mined. Game over.");
            break;
        }
    }

    socket_server.stop()?;

    stop_workers(&mut shared)?;

    while !children.is_empty() {
        match rx.recv() {
            Ok(Event::ChildExited { pid, status }) => {
                handle_child_exit(pid, status, &mut children, &shared)?;
            }

            Ok(Event::ReaperError(error)) => {
                return Err(io::Error::other(
                    format!("Child reaper failed during shutdown: {error}"),
                ));
            }

            Ok(Event::Socket(request)) => {
                let _ = request.response.send(
                    "Game is shutting down.".to_string()
                );
            }

            Ok(Event::Input(_))
            | Ok(Event::InputClosed)
            | Ok(Event::InputError(_)) => {}

            Err(_) => {
                return Err(io::Error::other(
                    "Event channel closed before all children exited",
                ));
            }
        }
    }

    game = read_shared_game(&shared)?;

    let remaining_minerals: i32 = game.map_minerals.iter().sum();

    println!(
        "Map minerals {}, player minerals {}, SCVs {}, Marines {}",
        remaining_minerals,
        game.player_minerals,
        game.scv_count,
        game.marine_count,
    );

    signal_handle.close();

    reaper_thread
        .join()
        .map_err(|_| io::Error::other("Child reaper thread panicked"))?;

    drop(input_thread);
    drop(tx);

    destroy_shared_game(shared)?;

    Ok(())
}

fn spawn_worker(shared: &SharedGame, id: i32) -> io::Result<Child> {
    let current_exe = std::env::current_exe()?;

    let directory = current_exe.parent().ok_or_else(|| {
        io::Error::other("Could not determine executable directory")
    })?;

    let worker_exe = directory.join("worker");

    let shared_name = shared.name.to_str().map_err(|_| {
        io::Error::other("Invalid shared-memory name")
    })?;

    Command::new(worker_exe)
        .arg(shared_name)
        .arg(id.to_string())
        .spawn()
}

fn reserve_unit(shared: &mut SharedGame, cost: i32, max_units: i32) -> io::Result<bool> {
    let state = shared.ptr;

    unsafe {
        lock_sem(&mut (*state).state_sem)?;

        let total_units =
            (*state).scv_count
            + (*state).marine_count
            + (*state).units_training;

        if (*state).player_minerals < cost || total_units >= max_units {
            unlock_sem(&mut (*state).state_sem)?;
            return Ok(false);
        }

        (*state).player_minerals -= cost;
        (*state).units_training += 1;

        unlock_sem(&mut (*state).state_sem)?;
    }

    Ok(true)
}

fn reserve_next_scv_id(shared: &mut SharedGame) -> io::Result<i32> {
    let state = shared.ptr;

    unsafe {
        lock_sem(&mut (*state).state_sem)?;

        let id = (*state).next_scv_id;
        (*state).next_scv_id += 1;

        unlock_sem(&mut (*state).state_sem)?;

        Ok(id)
    }
}

fn training_executable() -> io::Result<PathBuf> {
    let current_exe = std::env::current_exe()?;

    let directory = current_exe.parent().ok_or_else(|| {
        io::Error::other("Could not determine executable directory")
    })?;

    Ok(directory.join("training"))
}

fn register_child(children: &mut Children, child: Child, kind: ChildKind) {
    let pid = child.id() as libc::pid_t;

    children.insert(pid, ChildInfo { kind });

    drop(child);
}
fn spawn_marine_training() -> io::Result<Child> {
    Command::new(training_executable()?)
        .arg("marine")
        .spawn()
}

fn spawn_scv_training() -> io::Result<Child> {
    Command::new(training_executable()?)
        .arg("scv")
        .spawn()
}

fn finish_training(
    shared: &SharedGame,
    kind: ChildKind,
    succeeded: bool,
) -> io::Result<()> {
    let state = shared.ptr;

    unsafe {
        lock_sem(&mut (*state).state_sem)?;

        (*state).units_training =
            (*state).units_training.saturating_sub(1);

        if succeeded {
            match kind {
                ChildKind::MarineTraining => {
                    (*state).marine_count += 1;
                }
                ChildKind::ScvTraining { .. } => {
                    (*state).scv_count += 1;
                }
                _ => {
                    // This should not happen.
                    eprintln!("finish_training called with unexpected kind: {kind:?}");
                }
            }
        } else {
            (*state).player_minerals += UNIT_COST;
        }

        unlock_sem(&mut (*state).state_sem)?;
    }

    Ok(())
}

fn handle_child_exit(
    pid: libc::pid_t,
    status: i32,
    children: &mut Children,
    shared: &SharedGame,
) -> io::Result<()> {
    let Some(info) = children.remove(&pid) else {
        eprintln!("Received exit event for unknown child PID {pid}");
        return Ok(());
    };

    let succeeded =
        libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;

    match info.kind {
        ChildKind::Worker { id } => {
            if succeeded {
                println!("SCV {id} worker exited.");
            } else {
                eprintln!("SCV {id} worker failed or was terminated.");
            }
        }

        ChildKind::MarineTraining => {
            finish_training(
                shared,
                ChildKind::MarineTraining,
                succeeded,
            )?;

            if succeeded {
                println!("Marine ready.");
            } else {
                eprintln!("Marine training failed.");
            }
        }

        ChildKind::ScvTraining { id } => {
            finish_training(
                shared,
                ChildKind::ScvTraining { id },
                succeeded,
            )?;

            if succeeded {
                println!("SCV {id} ready.");

                let child = spawn_worker(shared, id)?;

                register_child(
                    children,
                    child,
                    ChildKind::Worker { id },
                );
            } else {
                eprintln!("SCV {id} training failed.");
            }
        }
    }

    Ok(())
}

fn read_shared_game(shared: &SharedGame) -> io::Result<GameState> {
    let state = shared.ptr;

    let (map_blocks, player_minerals, scv_count, marine_count) =
        unsafe {
            lock_sem(&mut (*state).state_sem)?;

            let result = (
                (*state).map_blocks,
                (*state).player_minerals,
                (*state).scv_count,
                (*state).marine_count,
            );

            unlock_sem(&mut (*state).state_sem)?;

            result
        };

    if map_blocks < 1 || map_blocks as usize > MAX_BLOCKS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid map_blocks value in shared memory",
        ));
    }

    let mut map_minerals = Vec::with_capacity(map_blocks as usize);

    for i in 0..map_blocks as usize {
        unsafe {
            lock_sem(&mut (*state).block_sems[i])?;

            let minerals = (*state).map_minerals[i];

            unlock_sem(&mut (*state).block_sems[i])?;

            map_minerals.push(minerals);
        }
    }

    Ok(GameState {
        map_minerals,
        player_minerals,
        scv_count,
        marine_count,
    })
}

fn stop_workers(shared: &mut SharedGame) -> io::Result<()> {
    request_shutdown(shared)
}

fn destroy_shared_game(shared: SharedGame) -> io::Result<()> {
    let state = shared.ptr;
    let map_blocks = unsafe { (*state).map_blocks as usize };

    let mut first_error: Option<io::Error> = None;

    unsafe {
        for i in 0..map_blocks {
            if libc::sem_destroy(&mut (*state).block_sems[i]) == -1 {
                if first_error.is_none() {
                    first_error = Some(last_os_error());
                }
            }
        }

        if libc::sem_destroy(&mut (*state).cc_sem) == -1 {
            if first_error.is_none() {
                first_error = Some(last_os_error());
            }
        }

        if libc::sem_destroy(&mut (*state).state_sem) == -1 {
            if first_error.is_none() {
                first_error = Some(last_os_error());
            }
        }

        if libc::munmap(state.cast::<libc::c_void>(), shared.size) == -1 {
            if first_error.is_none() {
                first_error = Some(last_os_error());
            }
        }

        if libc::shm_unlink(shared.name.as_ptr()) == -1 {
            if first_error.is_none() {
                first_error = Some(last_os_error());
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn last_os_error() -> io::Error {
    io::Error::last_os_error()
}

fn create_shared_game(game: &GameState, map_blocks: usize) -> io::Result<SharedGame> {
    if map_blocks == 0 || map_blocks > MAX_BLOCKS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("map_blocks must be between 1 and {MAX_BLOCKS}"),
        ));
    }

    if game.map_minerals.len() != map_blocks {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GameState mineral count does not match map_blocks",
        ));
    }

    let name = CString::new(format!(
        "/starcraft_{}",
        unsafe { libc::getpid() }
    ))
    .unwrap();

    let size = size_of::<SharedState>();

    let fd = unsafe {
        libc::shm_open(
            name.as_ptr(),
            libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
            0o600,
        )
    };

    if fd == -1 {
        return Err(last_os_error());
    }

    if unsafe { libc::ftruncate(fd, size as libc::off_t) } == -1 {
        let error = last_os_error();
        unsafe {
            libc::close(fd);
            libc::shm_unlink(name.as_ptr());
        }
        return Err(error);
    }

    let mapping = unsafe {
        libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };

    unsafe {
        libc::close(fd);
    }

    if mapping == libc::MAP_FAILED {
        let error = last_os_error();
        unsafe {
            libc::shm_unlink(name.as_ptr());
        }
        return Err(error);
    }

    let ptr = mapping.cast::<SharedState>();

    unsafe {
        ptr::write(
            ptr,
            SharedState {
                map_blocks: map_blocks as i32,
                map_minerals: [0; MAX_BLOCKS],
                player_minerals: game.player_minerals,
                scv_count: game.scv_count,
                marine_count: game.marine_count,
                units_training: 0,
                next_scv_id: game.scv_count + 1,
                shutdown: 0,

                state_sem: zeroed(),
                cc_sem: zeroed(),
                block_sems: zeroed(),
            },
        );

        for i in 0..map_blocks {
            (*ptr).map_minerals[i] = game.map_minerals[i];
        }
    }

    
    let init_result = unsafe {
        libc::sem_init(&mut (*ptr).state_sem, 1, 1)
    };
    
    if init_result == -1 {
        let error = last_os_error();
        unsafe {
            libc::munmap(mapping, size);
            libc::shm_unlink(name.as_ptr());
        }
        return Err(error);
    }
    
    if unsafe { libc::sem_init(&mut (*ptr).cc_sem, 1, 1) } == -1 {
        let error = last_os_error();
        
        unsafe {
            libc::sem_destroy(&mut (*ptr).state_sem);
            libc::munmap(mapping, size);
            libc::shm_unlink(name.as_ptr());
        }
        
        return Err(error);
    }
    
    let mut initialized_blocks = 0;
    for i in 0..map_blocks {
        if unsafe {
            libc::sem_init(&mut (*ptr).block_sems[i], 1, 1)
        } == -1 {
            let error = last_os_error();

            unsafe {
                for j in 0..initialized_blocks {
                    libc::sem_destroy(&mut (*ptr).block_sems[j]);
                }

                libc::sem_destroy(&mut (*ptr).cc_sem);
                libc::sem_destroy(&mut (*ptr).state_sem);
                libc::munmap(mapping, size);
                libc::shm_unlink(name.as_ptr());
            }

            return Err(error);
        }

        initialized_blocks += 1;
    }

    Ok(SharedGame { ptr, name, size })
}

fn lock_sem(sem: *mut libc::sem_t) -> io::Result<()> {
    loop {
        let result = unsafe { libc::sem_wait(sem) };

        if result == 0 {
            return Ok(());
        }

        let error = io::Error::last_os_error();

        // sem_wait can be interrupted by a signal.
        if error.raw_os_error() == Some(libc::EINTR) {
            continue;
        }

        return Err(error);
    }
}

fn unlock_sem(sem: *mut libc::sem_t) -> io::Result<()> {
    if unsafe { libc::sem_post(sem) } == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

fn request_shutdown(shared: &SharedGame) -> io::Result<()> {
    let state = shared.ptr;

    unsafe {
        lock_sem(&mut (*state).state_sem)?;

        (*state).shutdown = 1;

        unlock_sem(&mut (*state).state_sem)?;
    }

    Ok(())
}

fn handle_socket_request(
    request: ipc::SocketRequest,
    shared: &SharedGame,
) -> io::Result<bool> {
    match request.command {
        ipc::GameCommand::Status => {
            let game = read_shared_game(shared)?;

            let remaining_minerals: i32 = game.map_minerals.iter().sum();

            let response = format!(
                "Player minerals: {}\n\
                 Remaining minerals: {}\n\
                 SCVs: {}\n\
                 Marines: {}",
                game.player_minerals,
                remaining_minerals,
                game.scv_count,
                game.marine_count,
            );

            let _ = request.response.send(response);

            Ok(false)
        }

        ipc::GameCommand::Exit => {
            let _ = request.response.send(
                "Shutdown requested. The game is exiting.".to_string()
            );

            Ok(true)
        }
    }
}

// Event listening for signals from child processes and input from stdin.

pub(crate) enum Event {
    Input(String),
    InputClosed,
    InputError(String),

    ChildExited {
        pid: libc::pid_t,
        status: i32,
    },

    ReaperError(String),

    Socket(ipc::SocketRequest),
}

fn start_child_reaper(
    tx: Sender<Event>,
) -> io::Result<(SignalsHandle, JoinHandle<()>)> {
    let mut signals = Signals::new([SIGCHLD])?;
    let handle = signals.handle();

    let thread = thread::spawn(move || {
        for _signal in signals.forever() {
            loop {
                let mut status: libc::c_int = 0;

                let pid = unsafe {
                    libc::waitpid(-1, &mut status, libc::WNOHANG)
                };

                if pid > 0 {
                    if tx
                        .send(Event::ChildExited { pid, status })
                        .is_err()
                    {
                        return;
                    }

                    continue;
                }

                if pid == 0 {
                    break;
                }

                let error = io::Error::last_os_error();

                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }

                if error.raw_os_error() == Some(libc::ECHILD) {
                    break;
                }

                let _ = tx.send(Event::ReaperError(error.to_string()));
                return;
            }
        }
    });

    Ok((handle, thread))
}

fn start_input_reader(tx: Sender<Event>) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let mut line = String::new();

            match io::stdin().read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Event::InputClosed);
                    return;
                }

                Ok(_) => {
                    if tx.send(Event::Input(line)).is_err() {
                        return;
                    }
                }

                Err(error) => {
                    let _ = tx.send(Event::InputError(error.to_string()));
                    return;
                }
            }
        }
    })
}