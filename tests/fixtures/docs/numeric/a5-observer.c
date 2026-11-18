/* A5链接期只读报告观察器；不进入产品ABI，不修改/清空挂起错误。 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct { const unsigned char *ptr; size_t len; } XiaoA5Bytes;
typedef struct { uint64_t start, end; unsigned char present, reserved[7]; } XiaoA5Location;
typedef struct {
    uint32_t error_class, kind;
    uint64_t error_id;
    XiaoA5Bytes code, message_id;
    XiaoA5Location location;
    int32_t exit_code;
    uint32_t stack_depth, param_count;
} XiaoA5Snapshot;
extern int32_t xiao_runtime_error_snapshot(XiaoA5Snapshot *out);

/* 代替测试可执行文件的报告出口，不代替数值运算或错误传播。 */
void xiao_a5_test_report(void) {
    XiaoA5Snapshot snapshot = {0};
    if (xiao_runtime_error_snapshot(&snapshot) != 0 || !snapshot.location.present)
        exit(91);
    fprintf(stderr, "A5-ERROR code=%.*s span=%llu..%llu\n",
        (int)snapshot.code.len, (const char *)snapshot.code.ptr,
        (unsigned long long)snapshot.location.start,
        (unsigned long long)snapshot.location.end);
}
