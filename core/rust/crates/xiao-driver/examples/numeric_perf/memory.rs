//! F-3驻留内存读数，保留原始单位；不以Xiao对象计数代替OS峰值。

use serde::{Deserialize, Serialize};

/// OS原始读数与明确的字节换算。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Memory {
    pub method: String,
    pub unit: String,
    pub current_raw: u64,
    pub peak_raw: u64,
    pub current_bytes: u64,
    pub peak_bytes: u64,
}

#[cfg(target_os = "linux")]
/// /proc的VmRSS/VmHWM原始单位为KiB。
pub fn resident() -> Result<Memory, String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|error| error.to_string())?;
    let read = |name: &str| -> Result<u64, String> {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|line| line.split_whitespace().next())
            .ok_or_else(|| format!("缺少{name}"))?
            .parse::<u64>()
            .map_err(|error| error.to_string())
    };
    let current_raw = read("VmRSS:")?;
    let peak_raw = read("VmHWM:")?;
    Ok(Memory {
        method: "proc-vmrss-vmhwm".into(),
        unit: "KiB".into(),
        current_raw,
        peak_raw,
        current_bytes: current_raw * 1024,
        peak_bytes: peak_raw * 1024,
    })
}

#[cfg(target_os = "windows")]
/// WorkingSetSize与PeakWorkingSetSize的原始单位为字节。
pub fn resident() -> Result<Memory, String> {
    #[repr(C)]
    struct Counters {
        cb: u32,
        faults: u32,
        peak: usize,
        current: usize,
        other: [usize; 6],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        faults: 0,
        peak: 0,
        current: 0,
        other: [0; 6],
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<Counters>() as u32,
        )
    };
    if ok == 0 {
        return Err("GetProcessMemoryInfo失败".into());
    }
    let current_raw = counters.current as u64;
    let peak_raw = counters.peak as u64;
    Ok(Memory {
        method: "working-set/peak-working-set".into(),
        unit: "bytes".into(),
        current_raw,
        peak_raw,
        current_bytes: current_raw,
        peak_bytes: peak_raw,
    })
}

#[cfg(target_os = "macos")]
/// resident_size当前值和getrusage历史峰值；macOS ru_maxrss为字节。
pub fn resident() -> Result<Memory, String> {
    #[repr(C)]
    struct BasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        times: [i32; 4],
        policy: i32,
        suspend_count: i32,
    }
    unsafe extern "C" {
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    }
    let mut info = BasicInfo {
        virtual_size: 0,
        resident_size: 0,
        resident_size_max: 0,
        times: [0; 4],
        policy: 0,
        suspend_count: 0,
    };
    let mut count = (std::mem::size_of::<BasicInfo>() / 4) as u32;
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let ok = unsafe {
        task_info(
            mach_task_self_,
            20,
            (&mut info as *mut BasicInfo).cast(),
            &mut count,
        )
    };
    let used = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if ok != 0 || used != 0 {
        return Err("task_info/getrusage失败".into());
    }
    let peak_raw = unsafe { usage.assume_init() }.ru_maxrss as u64;
    Ok(Memory {
        method: "resident-size/getrusage-ru-maxrss".into(),
        unit: "bytes".into(),
        current_raw: info.resident_size,
        peak_raw,
        current_bytes: info.resident_size,
        peak_bytes: peak_raw,
    })
}
