pub(crate) enum Event {
    ChildExited {
        pid: libc::pid_t,
        status: libc::c_int,
    },

    ReaperError(String),
}
