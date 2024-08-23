#[derive(Clone, Copy)]
pub struct Event<Value> {
    pub time: std::time::Duration,
    pub task: [u8; crate::bpf_constants::TASK_COMM_LEN as usize],
    pub pid: u32,
    pub value: Value,
}

pub trait FromBytes<Value> {
    fn from_bytes(data: &[u8]) -> Event<Value>;
}

pub(crate) fn pid_to_name(pid: i32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or("?".to_string()).trim_end().to_string()
}

// An automatically-implemented "trait" for from_bytes in the typical case,
// i.e. we have a custom c-event type from the BPF skeleton, but similar
// conversion logic into a `crate::event::Event` struct. Can't (easily) be a
// trait because we don't control the c-event types. Implementing a "CEvent"
// trait with functions .task() -> String, .pid() -> u32, etc. would be another
// way to do this (so that we can constrain a FromBytes trait to a CEvent or
// something, but doing that adds a lot of boilerplate). So, just a macro.
#[macro_export]
macro_rules! impl_from_bytes_for {
    ($Prog:ty, $Value:ty, $c_event_default_func:expr) => {
        impl crate::event::FromBytes<$Value> for $Prog {
            fn from_bytes(data: &[u8]) -> crate::event::Event<$Value> {
                let mut event = $c_event_default_func();
                plain::copy_from_bytes(&mut event, data).expect("Data buffer was too short");
                crate::event::Event {
                    time: crate::time::elapsed_since_prog_start(),
                    task: event.task,
                    pid: event.pid as u32,
                    value: event.lat_us.into(),
                }
            }
        }
    };
}

pub(crate) use impl_from_bytes_for;
