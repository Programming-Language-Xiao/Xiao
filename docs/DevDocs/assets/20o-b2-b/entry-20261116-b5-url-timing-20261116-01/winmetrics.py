"""Windows telemetry adapter for a diagnostic resource-test child process."""
import ctypes
from ctypes import wintypes as w
import json
import os
import time


class FileTime(ctypes.Structure):
    _fields_ = [("low", w.DWORD), ("high", w.DWORD)]


class MemoryStatus(ctypes.Structure):
    _fields_ = [
        ("length", w.DWORD), ("load_percent", w.DWORD),
        ("total_physical", ctypes.c_ulonglong), ("available_physical", ctypes.c_ulonglong),
        ("total_pagefile", ctypes.c_ulonglong), ("available_pagefile", ctypes.c_ulonglong),
        ("total_virtual", ctypes.c_ulonglong), ("available_virtual", ctypes.c_ulonglong),
        ("available_extended_virtual", ctypes.c_ulonglong),
    ]


class ProcessMemory(ctypes.Structure):
    _fields_ = [
        ("cb", w.DWORD), ("page_faults", w.DWORD),
        ("peak_working_set", ctypes.c_size_t), ("working_set", ctypes.c_size_t),
        ("peak_paged_pool", ctypes.c_size_t), ("paged_pool", ctypes.c_size_t),
        ("peak_nonpaged_pool", ctypes.c_size_t), ("nonpaged_pool", ctypes.c_size_t),
        ("pagefile_usage", ctypes.c_size_t), ("peak_pagefile_usage", ctypes.c_size_t),
        ("private_usage", ctypes.c_size_t),
    ]


class CounterData(ctypes.Union):
    _fields_ = [("double_value", ctypes.c_double), ("long_value", w.LONG),
                ("large_value", ctypes.c_longlong)]


class CounterValue(ctypes.Structure):
    _fields_ = [("status", w.DWORD), ("data", CounterData)]


def ticks(value):
    return value.high * 4294967296 + value.low


class WindowsMetrics:
    """Keep process handles open and report raw observations and API statuses."""

    def __init__(self, pid):
        self.process = None
        self.query = None
        self.counters = {}
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.psapi = ctypes.WinDLL("psapi", use_last_error=True)
        self.pdh = ctypes.WinDLL("pdh", use_last_error=True)
        self.kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        self.kernel.OpenProcess.restype = w.HANDLE
        self.kernel.CloseHandle.argtypes = [w.HANDLE]
        self.kernel.CloseHandle.restype = w.BOOL
        self.kernel.GetProcessTimes.argtypes = [
            w.HANDLE, ctypes.POINTER(FileTime), ctypes.POINTER(FileTime),
            ctypes.POINTER(FileTime), ctypes.POINTER(FileTime),
        ]
        self.kernel.GetProcessTimes.restype = w.BOOL
        self.kernel.GetSystemTimes.argtypes = [ctypes.POINTER(FileTime)] * 3
        self.kernel.GetSystemTimes.restype = w.BOOL
        self.kernel.GlobalMemoryStatusEx.argtypes = [ctypes.POINTER(MemoryStatus)]
        self.kernel.GlobalMemoryStatusEx.restype = w.BOOL
        self.psapi.GetProcessMemoryInfo.argtypes = [
            w.HANDLE, ctypes.POINTER(ProcessMemory), w.DWORD,
        ]
        self.psapi.GetProcessMemoryInfo.restype = w.BOOL
        self.pdh.PdhOpenQueryW.argtypes = [
            w.LPCWSTR, ctypes.c_size_t, ctypes.POINTER(w.HANDLE),
        ]
        self.pdh.PdhOpenQueryW.restype = w.LONG
        self.pdh.PdhAddEnglishCounterW.argtypes = [
            w.HANDLE, w.LPCWSTR, ctypes.c_size_t, ctypes.POINTER(w.HANDLE),
        ]
        self.pdh.PdhAddEnglishCounterW.restype = w.LONG
        self.pdh.PdhCollectQueryData.argtypes = [w.HANDLE]
        self.pdh.PdhCollectQueryData.restype = w.LONG
        self.pdh.PdhGetFormattedCounterValue.argtypes = [
            w.HANDLE, w.DWORD, ctypes.POINTER(w.DWORD), ctypes.POINTER(CounterValue),
        ]
        self.pdh.PdhGetFormattedCounterValue.restype = w.LONG
        self.pdh.PdhCloseQuery.argtypes = [w.HANDLE]
        self.pdh.PdhCloseQuery.restype = w.LONG
        try:
            self.process = self.kernel.OpenProcess(0x0400 | 0x0010, False, pid)
            if not self.process:
                raise OSError(ctypes.get_last_error(), "OpenProcess failed")
            query = w.HANDLE()
            status = self.pdh.PdhOpenQueryW(None, 0, ctypes.byref(query))
            if status != 0:
                raise OSError(status, "PdhOpenQueryW failed")
            self.query = query
            for field, name in [
                ("processor_queue_length", "Processor Queue Length"),
                ("context_switches_per_second", "Context Switches/sec"),
            ]:
                counter = w.HANDLE()
                path = chr(92) + "System" + chr(92) + name
                status = self.pdh.PdhAddEnglishCounterW(
                    self.query, path, 0, ctypes.byref(counter))
                if status != 0:
                    raise OSError(status, "PdhAddEnglishCounterW failed: " + name)
                self.counters[field] = counter
        except Exception:
            self.close()
            raise

    def sample(self):
        row = {
            "observed_monotonic_ns": time.perf_counter_ns(),
            "observed_unix_ns": time.time_ns(),
            "process_cpu_100ns": None,
            "process_creation_100ns": None,
            "system_total_100ns": None,
            "system_idle_100ns": None,
            "system_busy_100ns": None,
            "working_set_bytes": None,
            "peak_working_set_bytes": None,
            "private_bytes": None,
            "page_fault_count": None,
            "available_physical_bytes": None,
            "total_physical_bytes": None,
            "memory_load_percent": None,
            "processor_queue_length": None,
            "context_switches_per_second": None,
            "api_status": {},
        }
        creation, exit_time, kernel, user = FileTime(), FileTime(), FileTime(), FileTime()
        valid = self.kernel.GetProcessTimes(
            self.process, ctypes.byref(creation), ctypes.byref(exit_time),
            ctypes.byref(kernel), ctypes.byref(user))
        row["api_status"]["process_times"] = bool(valid)
        if valid:
            row["process_cpu_100ns"] = ticks(kernel) + ticks(user)
            row["process_creation_100ns"] = ticks(creation)
        idle, kernel, user = FileTime(), FileTime(), FileTime()
        valid = self.kernel.GetSystemTimes(
            ctypes.byref(idle), ctypes.byref(kernel), ctypes.byref(user))
        row["api_status"]["system_times"] = bool(valid)
        if valid:
            total = ticks(kernel) + ticks(user)
            row["system_total_100ns"] = total
            row["system_idle_100ns"] = ticks(idle)
            row["system_busy_100ns"] = total - ticks(idle)
        memory = MemoryStatus()
        memory.length = ctypes.sizeof(memory)
        valid = self.kernel.GlobalMemoryStatusEx(ctypes.byref(memory))
        row["api_status"]["global_memory"] = bool(valid)
        if valid:
            row["available_physical_bytes"] = memory.available_physical
            row["total_physical_bytes"] = memory.total_physical
            row["memory_load_percent"] = memory.load_percent
        process_memory = ProcessMemory()
        process_memory.cb = ctypes.sizeof(process_memory)
        valid = self.psapi.GetProcessMemoryInfo(
            self.process, ctypes.byref(process_memory), process_memory.cb)
        row["api_status"]["process_memory"] = bool(valid)
        if valid:
            row["working_set_bytes"] = process_memory.working_set
            row["peak_working_set_bytes"] = process_memory.peak_working_set
            row["private_bytes"] = process_memory.private_usage
            row["page_fault_count"] = process_memory.page_faults
        collected = self.pdh.PdhCollectQueryData(self.query)
        row["api_status"]["pdh_collect"] = collected
        for field, counter in self.counters.items():
            value = CounterValue()
            status = self.pdh.PdhGetFormattedCounterValue(
                counter, 0x200, None, ctypes.byref(value))
            row["api_status"][field] = {
                "format_status": status, "counter_status": value.status,
            }
            if collected == 0 and status == 0 and value.status in (0, 1):
                row[field] = value.data.double_value
        return row

    def close(self):
        result = {}
        if self.query:
            result["pdh_close_status"] = self.pdh.PdhCloseQuery(self.query)
            self.query = None
        if self.process:
            result["process_handle_closed"] = bool(self.kernel.CloseHandle(self.process))
            self.process = None
        return result


if __name__ == "__main__":
    metrics = WindowsMetrics(os.getpid())
    try:
        first = metrics.sample()
        time.sleep(0.6)
        second = metrics.sample()
        print(json.dumps({"preflight_only": True, "samples": [first, second]},
                         ensure_ascii=False, allow_nan=False))
    finally:
        metrics.close()
