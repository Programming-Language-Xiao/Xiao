/* A5测试链接shim：旁观真实cast/release，或定向返回一次既有011。 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { uint32_t tag; uint64_t payload; } XiaoA5Value;
typedef struct { const unsigned char *ptr; size_t len; } XiaoA5Bytes;
typedef struct { uint64_t start, end; unsigned char present, reserved[7]; } XiaoA5Location;
extern int32_t xiao_runtime_value_cast(XiaoA5Bytes, const XiaoA5Value *, XiaoA5Value *);
extern void xiao_runtime_value_release_strong(XiaoA5Value *);
extern void xiao_runtime_value_release_any(XiaoA5Value *);
extern int32_t xiao_runtime_error_raise_type(XiaoA5Bytes, XiaoA5Bytes, XiaoA5Bytes, XiaoA5Location);
static XiaoA5Value selected;
static unsigned releases, resumed, injected;

static void fail(const char *message) { fputs(message, stderr); exit(91); }
static int is_str(XiaoA5Bytes name) { return name.len == 3 && memcmp(name.ptr, "str", 3) == 0; }

#ifdef A5_ALLOCATION_FAILURE
static void verify(void) { if (injected != 1) fail("A5 allocation injection missing\n"); }
#else
static void verify(void) {
    if (!selected.payload || resumed != 1 || releases != 1)
        fail("A5 selected temporary was not released exactly once\n");
}
#endif

int32_t xiao_a5_test_cast(XiaoA5Bytes target, const XiaoA5Value *value, XiaoA5Value *out) {
#ifdef A5_ALLOCATION_FAILURE
    if (is_str(target) && !injected) {
        const XiaoA5Value before = *out, source = *value;
        const unsigned char type[] = "ResourceError", code[] = "X06-RUNTIME-011", message[] = "A5 controlled result allocation failure";
        XiaoA5Bytes t = {type, sizeof(type)-1}, c = {code, sizeof(code)-1}, m = {message, sizeof(message)-1};
        XiaoA5Location location = {0};
        if (atexit(verify) != 0) fail("A5 cannot install allocation observer\n");
        if (xiao_runtime_error_raise_type(t, c, m, location) != 0)
            fail("A5 cannot raise controlled 011\n");
        if (out->tag != before.tag || out->payload != before.payload || value->tag != source.tag || value->payload != source.payload)
            fail("A5 failure changed source or out\n");
        injected++;
        return 6;
    }
#else
    if (is_str(target) && selected.payload && !resumed) {
        if (releases != 1) fail("A5 temporary survived into the next expression\n");
        resumed = 1;
    }
#endif
    int32_t status = xiao_runtime_value_cast(target, value, out);
#ifndef A5_ALLOCATION_FAILURE
    if (is_str(target) && status == 0 && !selected.payload) {
        selected = *out;
        if (atexit(verify) != 0) fail("A5 cannot install lifetime observer\n");
    }
#endif
    return status;
}

static void observe(const XiaoA5Value *value) {
    if (selected.payload && !resumed && value->tag == selected.tag && value->payload == selected.payload)
        releases++;
}
void xiao_a5_test_release_strong(XiaoA5Value *value) { observe(value); xiao_runtime_value_release_strong(value); }
void xiao_a5_test_release_any(XiaoA5Value *value) { observe(value); xiao_runtime_value_release_any(value); }
