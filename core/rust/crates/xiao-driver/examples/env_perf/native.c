/* B3-4E 示例进程。通过现有嵌入入口为每次负载创建新环境。 */
#if defined(__APPLE__)
#define _DARWIN_C_SOURCE 1
#elif !defined(_WIN32)
#define _POSIX_C_SOURCE 200809L
#endif
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if defined(_WIN32)
#include <windows.h>
#include <psapi.h>
#else
#include <time.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <mach/mach.h>
#include <sys/resource.h>
#endif
#endif

typedef struct {
    uint32_t tag;
    uint64_t payload;
} XiaoValue;

extern int32_t xiao_application_main(void);
extern int32_t xiao_runtime_env_run_injected(int32_t (*entry)(void), const uint8_t *data,
                                            size_t len, uint64_t capabilities);

static uint64_t operation_ns;
static int observed;
static int measure_snapshot;
static double snapshot_start;
static double snapshot_seconds;

static void fail(const char *message) {
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static double clock_seconds(void) {
#if defined(_WIN32)
    LARGE_INTEGER frequency, ticks;
    if (!QueryPerformanceFrequency(&frequency) || !QueryPerformanceCounter(&ticks)) {
        fail("QueryPerformanceCounter failed");
    }
    return (double)ticks.QuadPart / (double)frequency.QuadPart;
#else
    struct timespec ticks;
    if (clock_gettime(CLOCK_MONOTONIC, &ticks) != 0) fail("clock_gettime failed");
    return (double)ticks.tv_sec + (double)ticks.tv_nsec / 1e9;
#endif
}

static void memory_json(void) {
    uint64_t current = 0, peak = 0;
#if defined(_WIN32)
    const char *method = "working-set/peak-working-set", *unit = "bytes";
    const uint64_t factor = 1;
    PROCESS_MEMORY_COUNTERS counters;
    if (GetProcessMemoryInfo(GetCurrentProcess(), &counters, sizeof(counters))) {
        current = (uint64_t)counters.WorkingSetSize;
        peak = (uint64_t)counters.PeakWorkingSetSize;
    }
#elif defined(__APPLE__)
    const char *method = "resident-size/getrusage-ru-maxrss", *unit = "bytes";
    const uint64_t factor = 1;
    struct mach_task_basic_info info;
    mach_msg_type_number_t count = MACH_TASK_BASIC_INFO_COUNT;
    struct rusage usage;
    if (task_info(mach_task_self(), MACH_TASK_BASIC_INFO, (task_info_t)&info, &count) == KERN_SUCCESS
        && getrusage(RUSAGE_SELF, &usage) == 0 && usage.ru_maxrss >= 0) {
        current = (uint64_t)info.resident_size;
        peak = (uint64_t)usage.ru_maxrss;
    }
#else
    const char *method = "proc-vmrss-vmhwm", *unit = "KiB";
    const uint64_t factor = 1024;
    FILE *status = fopen("/proc/self/status", "r");
    if (status) {
        char line[256];
        unsigned long long rss = 0, hwm = 0;
        while (fgets(line, sizeof(line), status)) {
            if (sscanf(line, "VmRSS: %llu kB", &rss) == 1) continue;
            (void)sscanf(line, "VmHWM: %llu kB", &hwm);
        }
        fclose(status);
        current = (uint64_t)rss;
        peak = (uint64_t)hwm;
    }
#endif
    if (current == 0 || peak < current) {
        fputs("null", stdout);
        return;
    }
    printf("{\"method\":\"%s\",\"unit\":\"%s\",\"current_raw\":%llu,\"peak_raw\":%llu,"
           "\"current_bytes\":%llu,\"peak_bytes\":%llu}", method, unit,
           (unsigned long long)current, (unsigned long long)peak,
           (unsigned long long)(current * factor), (unsigned long long)(peak * factor));
}

/* 语言程序只输出一个 Int，值为实测纳秒或黄金验证失败的 -1。 */
int32_t xiao_b3_measure_print(const XiaoValue *values, size_t length, uint64_t capabilities) {
    if (observed || !values || length != 1 || capabilities != XIAO_B3_MASK
        || values[0].tag != 2 || values[0].payload > INT64_MAX) {
        fail("unexpected Xiao output or env validation failed");
    }
    observed = 1;
    operation_ns = values[0].payload;
    return 0;
}

static int32_t run_language(void) {
    if (measure_snapshot) snapshot_seconds = clock_seconds() - snapshot_start;
    return xiao_application_main();
}

int main(int argc, char **argv) {
    if (argc != 2) fail("expected one workload argument");
    measure_snapshot = strcmp(argv[1], "snapshot") == 0;
#if defined(_WIN32)
    unsigned long pid = (unsigned long)GetCurrentProcessId();
#else
    unsigned long pid = (unsigned long)getpid();
#endif
    printf("{\"event\":\"baseline\",\"pid\":%lu,\"memory\":", pid);
    memory_json();
    puts("}");
    fflush(stdout);

    FILE *file = fopen("snapshot.json", "rb");
    if (!file || fseek(file, 0, SEEK_END) != 0) fail("cannot open snapshot");
    long length = ftell(file);
    if (length <= 0 || fseek(file, 0, SEEK_SET) != 0) fail("invalid snapshot length");
    uint8_t *data = malloc((size_t)length);
    if (!data || fread(data, 1, (size_t)length, file) != (size_t)length) fail("cannot read snapshot");
    fclose(file);

    unsigned repeats;
    while (scanf("%u", &repeats) == 1 && repeats != 0) {
        double seconds = 0;
        for (unsigned i = 0; i < repeats; ++i) {
            observed = 0;
            if (measure_snapshot) snapshot_start = clock_seconds();
            int32_t status = xiao_runtime_env_run_injected(run_language, data, (size_t)length,
                                                          XIAO_B3_MASK);
            if (status != 0) {
                free(data);
                return status;
            }
            if (!observed) fail("the workload did not report its validation");
            seconds += measure_snapshot ? snapshot_seconds : (double)operation_ns / 1e9;
        }
        printf("{\"event\":\"sample\",\"repeats\":%u,\"seconds\":%.12f,\"memory\":", repeats, seconds);
        memory_json();
        puts("}");
        fflush(stdout);
    }
    free(data);
    return 0;
}
