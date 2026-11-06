//! F-3驻留内存读数，保留原始单位；不以Xiao对象计数代替OS峰值。

use serde::{Deserialize, Serialize};

/// OS原始读数与明确的字节换算。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Memory {
    /// 平台OS驻留内存数据来源。
    pub method: String,
    /// 原始读数单位，仅bytes或KiB。
    pub unit: String,
    /// 同PID当前驻留量的原始读数。
    pub current_raw: u64,
    /// 同PID历史驻留峰值的原始读数。
    pub peak_raw: u64,
    /// 由明确原始单位换算的当前字节数。
    pub current_bytes: u64,
    /// 由明确原始单位换算的峰值字节数。
    pub peak_bytes: u64,
}

impl Memory {
    /// 无效OS读数必须缺测；不能以零值或反向高水位伪造低内存结果。
    fn checked(self) -> Result<Self, String> {
        if self.current_raw == 0 || self.peak_raw < self.current_raw {
            Err("OS驻留内存读数为零或峰值小于当前值".into())
        } else {
            Ok(self)
        }
    }
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
    Memory {
        method: "proc-vmrss-vmhwm".into(),
        unit: "KiB".into(),
        current_raw,
        peak_raw,
        current_bytes: current_raw * 1024,
        peak_bytes: peak_raw * 1024,
    }
    .checked()
}

#[cfg(target_os = "windows")]
/// WorkingSetSize与PeakWorkingSetSize的原始单位为字节。
pub fn resident() -> Result<Memory, String> {
    #[repr(C)]
    /// 与Windows PROCESS_MEMORY_COUNTERS布局匹配的只读计量结构。
    struct Counters {
        /// 调用方提供的结构大小。
        cb: u32,
        /// OS返回的页错误计数，本协议不作为内存值。
        faults: u32,
        /// 当前进程历史峰值工作集字节数。
        peak: usize,
        /// 当前进程工作集字节数。
        current: usize,
        /// ABI其余计数字段，只用于维持既有系统结构布局。
        other: [usize; 6],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        /// 获取当前进程伪句柄，不创建可关闭资源。
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        /// 读取当前进程工作集与历史峰值，失败返回零。
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
    Memory {
        method: "working-set/peak-working-set".into(),
        unit: "bytes".into(),
        current_raw,
        peak_raw,
        current_bytes: current_raw,
        peak_bytes: peak_raw,
    }
    .checked()
}

#[cfg(target_os = "macos")]
/// resident_size当前值和getrusage历史峰值；macOS ru_maxrss为字节。
pub fn resident() -> Result<Memory, String> {
    #[repr(C)]
    /// 与macOS MACH_TASK_BASIC_INFO布局匹配的当前驻留视图。
    struct BasicInfo {
        /// OS返回的虚拟地址空间量，不作为本协议RSS。
        virtual_size: u64,
        /// 当前任务驻留字节数。
        resident_size: u64,
        /// 系统结构中的驻留上界，峰值仍由getrusage读取。
        resident_size_max: u64,
        /// 用户和系统时间字段，用于匹配系统结构布局。
        times: [i32; 4],
        /// 系统调度策略字段，不参与数值测量。
        policy: i32,
        /// 系统挂起计数字段，不参与数值测量。
        suspend_count: i32,
    }
    unsafe extern "C" {
        /// 当前任务的Mach端口，由系统初始化。
        static mach_task_self_: u32;
        /// 读取当前任务的基础内存结构，成功返回零。
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
    let peak_raw = u64::try_from(unsafe { usage.assume_init() }.ru_maxrss)
        .map_err(|_| "getrusage返回负峰值")?;
    Memory {
        method: "resident-size/getrusage-ru-maxrss".into(),
        unit: "bytes".into(),
        current_raw: info.resident_size,
        peak_raw,
        current_bytes: info.resident_size,
        peak_bytes: peak_raw,
    }
    .checked()
}
