/* 测试可执行文件专用：不进入 Runtime，不新增生产 ABI 或环境变量注入协议。 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct { uint32_t tag; uint64_t payload; } XiaoTestValue;
typedef struct { const unsigned char *ptr; size_t len; } XiaoTestBytes;
typedef struct { uint64_t start, end; unsigned char present, reserved[7]; } XiaoTestLocation;

extern int32_t xiao_runtime_error_code(const XiaoTestValue *, XiaoTestBytes *);
extern int32_t xiao_runtime_value_copy(const XiaoTestValue *, XiaoTestValue *);
extern int32_t xiao_runtime_string_new(XiaoTestBytes, void **);
extern void xiao_runtime_value_release_strong(XiaoTestValue *);
extern void xiao_runtime_value_release_any(XiaoTestValue *);
extern int32_t xiao_runtime_error_raise_type(XiaoTestBytes, XiaoTestBytes, XiaoTestBytes, XiaoTestLocation);

static uint64_t selected_error;
static unsigned temporary_releases;
static unsigned resumed_source;

static void fail(const char *message) {
    fputs(message, stderr);
    _Exit(91);
}

static void verify_cleanup(void) {
    if (!selected_error || !resumed_source || temporary_releases != 1)
        fail("FIX-B temporary error reference was not released exactly once\n");
}

int32_t xiao_test_error_code(const XiaoTestValue *value, XiaoTestBytes *out) {
    if (!value || value->tag != 16 || selected_error)
        fail("FIX-B test getter did not receive its first valid source\n");
    selected_error = value->payload;
    if (atexit(verify_cleanup) != 0) fail("FIX-B cannot install test cleanup check\n");
#ifdef FIX_B_GETTER_FAILURE
    (void)out;
    return 3;
#else
    return xiao_runtime_error_code(value, out);
#endif
}

int32_t xiao_test_string_new(XiaoTestBytes input, void **out) {
    /* 复制开始前，参数临时引用必须仍在；不能先 release 再使用借用视图。 */
    if (!selected_error || resumed_source || temporary_releases)
        fail("FIX-B source was released before its copy completed\n");
#ifdef FIX_B_SUCCESS
    return xiao_runtime_string_new(input, out);
#else
    (void)input;
    (void)out;
    static const unsigned char type[] = "ResourceError";
    static const unsigned char code[] = "X06-RUNTIME-011";
    static const unsigned char message[] = "test-only result copy failure";
    XiaoTestBytes t = {type, sizeof(type) - 1};
    XiaoTestBytes c = {code, sizeof(code) - 1};
    XiaoTestBytes m = {message, sizeof(message) - 1};
    XiaoTestLocation location = {0};
    if (xiao_runtime_error_raise_type(t, c, m, location) != 0)
        fail("FIX-B cannot construct test-only allocation failure\n");
    return 6;
#endif
}

int32_t xiao_test_value_copy(const XiaoTestValue *value, XiaoTestValue *out) {
    if (selected_error && !resumed_source && value->tag == 16 && value->payload == selected_error) {
        /* 在下一次读取原错误之前，失败调用的临时引用恰好归还一次。 */
        if (temporary_releases != 1) fail("FIX-B failed-path cleanup count mismatch\n");
        resumed_source = 1;
    }
    return xiao_runtime_value_copy(value, out);
}

static void observe_release(const XiaoTestValue *value) {
    if (selected_error && !resumed_source && value->tag == 16 && value->payload == selected_error)
        temporary_releases++;
}

void xiao_test_release_strong(XiaoTestValue *value) {
    observe_release(value);
    xiao_runtime_value_release_strong(value);
}

void xiao_test_release_any(XiaoTestValue *value) {
    observe_release(value);
    xiao_runtime_value_release_any(value);
}
