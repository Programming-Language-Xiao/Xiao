/* A6测量可执行文件专用；不进入Runtime，不新增生产ABI或intrinsic。 */
#if defined(__APPLE__)
/* macOS的rusage扩展字段需要Darwin可见性；严格POSIX会隐藏ru_maxrss。 */
#define _DARWIN_C_SOURCE 1
#elif !defined(_WIN32)
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
typedef struct { uint32_t tag; uint64_t payload; } XiaoA6Value;
static double started;
static int active, baseline, warmed;
static void fail(void) { fputs("A6 marker/golden/probe failure\n",stderr); exit(93); }
static unsigned long pid(void) {
#if defined(_WIN32)
    return (unsigned long)GetCurrentProcessId();
#else
    return (unsigned long)getpid();
#endif
}
static double now(void) {
#if defined(_WIN32)
    LARGE_INTEGER value, frequency;
    if (!QueryPerformanceCounter(&value)||!QueryPerformanceFrequency(&frequency)) fail();
    return (double)value.QuadPart/(double)frequency.QuadPart;
#else
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC,&value)!=0) fail();
    return (double)value.tv_sec+(double)value.tv_nsec/1e9;
#endif
}
static void memory(void) {
    uint64_t current=0, peak=0;
#if defined(_WIN32)
    PROCESS_MEMORY_COUNTERS counters;
    if (!GetProcessMemoryInfo(GetCurrentProcess(),&counters,sizeof(counters))) {fputs("null",stdout);return;}
    current=(uint64_t)counters.WorkingSetSize;peak=(uint64_t)counters.PeakWorkingSetSize;
    const char *method="working-set/peak-working-set", *unit="bytes";
    const uint64_t factor=1;
#elif defined(__APPLE__)
    struct mach_task_basic_info info;
    mach_msg_type_number_t count=MACH_TASK_BASIC_INFO_COUNT;
    struct rusage usage;
    if (task_info(mach_task_self(),MACH_TASK_BASIC_INFO,(task_info_t)&info,&count)!=KERN_SUCCESS || getrusage(RUSAGE_SELF,&usage)!=0) {fputs("null",stdout);return;}
    current=(uint64_t)info.resident_size;peak=(uint64_t)usage.ru_maxrss;
    const char *method="resident-size/getrusage-ru-maxrss", *unit="bytes";
    const uint64_t factor=1;
#else
    char line[256]; unsigned long long resident=0, high=0;
    FILE *file=fopen("/proc/self/status","r");
    if (!file) {fputs("null",stdout);return;}
    while (fgets(line,sizeof(line),file)) {
        if (sscanf(line,"VmRSS: %llu kB",&resident)==1) continue;
        (void)sscanf(line,"VmHWM: %llu kB",&high);
    }
    fclose(file);current=(uint64_t)resident;peak=(uint64_t)high;
    const char *method="proc-vmrss-vmhwm", *unit="KiB";
    const uint64_t factor=1024;
#endif
    if (!current || peak<current) {fputs("null",stdout);return;}
    printf("{\"method\":\"%s\",\"unit\":\"%s\",\"current_raw\":%llu,\"peak_raw\":%llu,\"current_bytes\":%llu,\"peak_bytes\":%llu}",method,unit,(unsigned long long)current,(unsigned long long)peak,(unsigned long long)(current*factor),(unsigned long long)(peak*factor));
}
int32_t xiao_a6_measure_print(const XiaoA6Value *values,size_t length,uint64_t capabilities) {
    (void)capabilities;
    if (length!=1 || !values) fail();
    if (values[0].tag==1) {
        /* Bool的既有ABI只承诺u8；不能读取union未定义的高字节。 */
        if (active || (uint8_t)values[0].payload!=1) fail();
        puts("{\"event\":\"validated\"}");fflush(stdout);return 0;
    }
    if (values[0].tag!=2) fail();
    switch (values[0].payload) {
    case 0:
        if (baseline || active) fail();baseline=1;
        printf("{\"event\":\"baseline\",\"pid\":%lu,\"memory\":",pid());memory();puts("}");break;
    case 1:
        if (!baseline || warmed || active) fail();warmed=1;
        printf("{\"event\":\"ready\",\"pid\":%lu}\n",pid());break;
    case 2:
        if (!warmed || active) fail();active=1;started=now();break;
    case 3: {
        if (!active) fail();double elapsed=now()-started;active=0;
        printf("{\"event\":\"sample\",\"seconds\":%.9f,\"memory\":",elapsed);memory();puts("}");break;
    }
    case 4:
        if (active || !warmed) fail();puts("{\"event\":\"done\"}");break;
    default:fail();
    }
    fflush(stdout);return 0;
}
