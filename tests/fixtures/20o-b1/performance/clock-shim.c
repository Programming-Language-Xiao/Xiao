/* 仅测量可执行文件使用，不进入 Runtime，不增加生产 ABI。 */
#if !defined(_WIN32)
#define _POSIX_C_SOURCE 200809L
#endif
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#if defined(_WIN32)
#include <windows.h>
#include <psapi.h>
#else
#include <time.h>
#include <sys/resource.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <mach/mach.h>
#endif
#endif

typedef struct { uint32_t tag; uint64_t payload; } XiaoMeasureValue;
extern int32_t xiao_runtime_print_values_with_capabilities(const XiaoMeasureValue *, size_t, uint64_t);
static double started;
static uint64_t baseline;
static int active;

static void fail(void) { fputs("B1 measurement marker failure\n", stderr); exit(93); }
static double now(void) {
#if defined(_WIN32)
    LARGE_INTEGER value, frequency;
    if (!QueryPerformanceCounter(&value) || !QueryPerformanceFrequency(&frequency)) fail();
    return (double)value.QuadPart / (double)frequency.QuadPart;
#else
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC, &value) != 0) fail();
    return (double)value.tv_sec + (double)value.tv_nsec / 1e9;
#endif
}
static void memory(uint64_t *current, uint64_t *peak) {
#if defined(_WIN32)
    PROCESS_MEMORY_COUNTERS counters;
    if (!GetProcessMemoryInfo(GetCurrentProcess(), &counters, sizeof(counters))) fail();
    *current = (uint64_t)counters.WorkingSetSize; *peak = (uint64_t)counters.PeakWorkingSetSize;
#elif defined(__APPLE__)
    struct mach_task_basic_info info;
    mach_msg_type_number_t count = MACH_TASK_BASIC_INFO_COUNT;
    if (task_info(mach_task_self(), MACH_TASK_BASIC_INFO, (task_info_t)&info, &count) != KERN_SUCCESS) fail();
    *current = (uint64_t)info.resident_size; *peak = (uint64_t)info.resident_size_max;
#else
    char line[256];
    unsigned long long resident = 0, high = 0;
    FILE *file = fopen("/proc/self/status", "r");
    if (!file) fail();
    while (fgets(line, sizeof(line), file)) {
        if (sscanf(line, "VmRSS: %llu kB", &resident) == 1) continue;
        (void)sscanf(line, "VmHWM: %llu kB", &high);
    }
    fclose(file);
    if (!resident || high < resident) fail();
    *current = (uint64_t)resident * 1024;
    *peak = (uint64_t)high * 1024;
#endif
}

int32_t xiao_b1_measure_print(const XiaoMeasureValue *values, size_t length, uint64_t capabilities) {
    /* Int 的稳定 tag 为 2，marker 不读取任何不透明句柄。 */
    if (length == 1 && values && values[0].tag == 2 && values[0].payload <= 1) {
        if (values[0].payload == 0) {
            uint64_t ignored;
            if (active) fail();
            memory(&baseline, &ignored);
            started = now(); active = 1;
        } else {
            if (!active) fail();
            double elapsed = now() - started;
            uint64_t current, peak;
            memory(&current, &peak);
            active = 0;
            printf("B1-PERF-SAMPLE %.9f %llu\n", elapsed, (unsigned long long)(peak > baseline ? peak - baseline : 0));
        }
        return 0;
    }
    if (active) fail();
    return xiao_runtime_print_values_with_capabilities(values, length, capabilities);
}
