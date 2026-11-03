
use std::ffi::CString;
use std::io;
use std::mem::size_of;
use std::ptr;
use std::thread::sleep;
use std::time::Duration;

use libc;

const MAX_BLOCKS: usize = 128;
const MINERALS_PER_TRIP: i32 = 8;
const TRAVEL_TIME: Duration = Duration::from_secs(3);
const UNLOAD_TIME: Duration = Duration::from_secs(2);

#[repr(C)]
struct SharedState {
    map_blocks: i32,
    map_minerals: [i32; MAX_BLOCKS],

    player_minerals: i32,
    scv_count: i32,
    marine_count: i32,
    units_training: i32,
    next_scv_id: i32,

    shutdown: i32,

    state_sem: libc::sem_t,
    cc_sem: libc::sem_t,
    block_sems: [libc::sem_t; MAX_BLOCKS],
}

struct SharedGame {
    ptr: *mut SharedState,
    size: usize,
}

fn main() {
    if let Err(error) = run_worker() {
        eprintln!("Worker error: {error}");
        std::process::exit(1);
    }
}

fn run_worker() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let shared_name = args.get(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: worker <shared-memory-name> <scv-id>",
        )
    })?;

    let id: i32 = args
        .get(2)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Missing SCV ID")
        })?
        .parse()
        .map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid SCV ID")
        })?;

    let shared_name = CString::new(shared_name.as_str()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Shared-memory name contains a null byte",
        )
    })?;

    let shared = open_shared_game(&shared_name)?;

    println!("SCV {id} worker started.");

    let result = mine_loop(&shared, id);

    let cleanup_result = unsafe {
        libc::munmap(shared.ptr.cast(), shared.size)
    };

    if cleanup_result == -1 {
        let error = io::Error::last_os_error();

        if result.is_ok() {
            return Err(error);
        }

        eprintln!("SCV {id}: failed to unmap shared memory: {error}");
    }

    result
}

fn open_shared_game(name: &CString) -> io::Result<SharedGame> {
    let size = size_of::<SharedState>();

    let fd = unsafe {
        libc::shm_open(name.as_ptr(), libc::O_RDWR, 0)
    };

    if fd == -1 {
        return Err(io::Error::last_os_error());
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

    let saved_error = if mapping == libc::MAP_FAILED {
        Some(io::Error::last_os_error())
    } else {
        None
    };

    unsafe {
        libc::close(fd);
    }

    if let Some(error) = saved_error {
        return Err(error);
    }

    Ok(SharedGame {
        ptr: mapping.cast::<SharedState>(),
        size,
    })
}

fn mine_loop(shared: &SharedGame, id: i32) -> io::Result<()> {
    loop {
        if shutdown_requested(shared)? {
            break;
        }

        let (block_index, minerals) = take_from_any_block(shared)?;

        if minerals == 0 {
            println!("SCV {id}: all mineral blocks are empty.");
            break;
        }

        println!(
            "SCV {id} mined {minerals} minerals from block {block_index}."
        );

        sleep(TRAVEL_TIME);

        unsafe {
            lock_sem(&mut (*shared.ptr).cc_sem)?;
        }

        sleep(UNLOAD_TIME);

        let deposit_result = unsafe {
            let state = shared.ptr;

            lock_sem(&mut (*state).state_sem)?;

            (*state).player_minerals += minerals;

            let balance = (*state).player_minerals;

            let unlock_result = unlock_sem(&mut (*state).state_sem);

            unlock_result.map(|()| balance)
        };

        let cc_unlock_result = unsafe {
            unlock_sem(&mut (*shared.ptr).cc_sem)
        };

        let balance = deposit_result?;
        cc_unlock_result?;

        println!(
            "SCV {id} delivered {minerals} minerals. Player minerals: {balance}."
        );
    }

    println!("SCV {id} worker stopped.");

    Ok(())
}

fn shutdown_requested(shared: &SharedGame) -> io::Result<bool> {
    unsafe {
        let state = shared.ptr;

        lock_sem(&mut (*state).state_sem)?;

        let shutdown = (*state).shutdown != 0;

        unlock_sem(&mut (*state).state_sem)?;

        Ok(shutdown)
    }
}

fn take_from_any_block(shared: &SharedGame) -> io::Result<(usize, i32)> {
    let state = shared.ptr;

    let block_count = unsafe {
        (*state).map_blocks as usize
    };

    for block_index in 0..block_count {
        unsafe {
            let block_sem = &mut (*state).block_sems[block_index];

            lock_sem(block_sem)?;

            let available = (*state).map_minerals[block_index];

            let mined = available.min(MINERALS_PER_TRIP);

            (*state).map_minerals[block_index] -= mined;

            let unlock_result = unlock_sem(block_sem);

            unlock_result?;

            if mined > 0 {
                return Ok((block_index, mined));
            }
        }
    }

    Ok((0, 0))
}

fn lock_sem(sem: *mut libc::sem_t) -> io::Result<()> {
    loop {
        let result = unsafe {
            libc::sem_wait(sem)
        };

        if result == 0 {
            return Ok(());
        }

        let error = io::Error::last_os_error();

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
