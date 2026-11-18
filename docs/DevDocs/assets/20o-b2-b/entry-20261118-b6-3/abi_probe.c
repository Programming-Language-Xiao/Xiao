/* Standalone evidence client for the fixed Runtime C ABI. */
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { const unsigned char *ptr; size_t len; } Bytes;
typedef struct { unsigned char *ptr; size_t capacity; } Buffer;
typedef union { uint64_t raw; int64_t integer; double real; void *handle; } Payload;
typedef struct { uint32_t tag; Payload payload; } Value;
typedef struct { uint64_t start, end; uint8_t present, reserved[7]; } Location;
typedef struct {
    uint32_t class, kind;
    uint64_t id;
    Bytes code, message_id;
    Location location;
    int32_t exit_code;
    uint32_t stack_depth, param_count;
} Snapshot;

_Static_assert(sizeof(Value) == 16 && offsetof(Value, payload) == 8, "XiaoValue layout");
_Static_assert(sizeof(Location) == 24 && sizeof(Snapshot) == 88, "XiaoErrorSnapshot layout");

enum { INT = 2, FLOAT = 5, LFLOAT = 7, STR = 8, UNKNOWN = 999 };

extern uint32_t xiao_runtime_abi_version(void);
extern uint32_t xiao_runtime_abi_minor_version(void);
extern int32_t xiao_runtime_string_new(Bytes, void **);
extern int32_t xiao_runtime_string_copy(void *, Buffer, size_t *);
extern void xiao_runtime_value_release(Value *);
extern void xiao_runtime_release(void *);
extern void *xiao_runtime_weak(void *);
extern void *xiao_runtime_weak_upgrade(void *);
extern void xiao_runtime_weak_release(void *);
extern int32_t xiao_runtime_error_raise_type(Bytes, Bytes, Bytes, Location);
extern int32_t xiao_runtime_error_snapshot(Snapshot *);
extern int32_t xiao_runtime_error_take(Value *);
extern int32_t xiao_runtime_error_matches(Bytes);
extern void xiao_runtime_error_clear(void);
extern void xiao_runtime_error_report(void);
extern int32_t xiao_runtime_error_exit_code(void);
extern int32_t xiao_runtime_lfloat_divide(const Value *, const Value *, const Value *, const Value *, Value *);

static Bytes bytes(const char *text) {
    return (Bytes){(const unsigned char *)text, strlen(text)};
}

static void require(int condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "B6-3 ABI observation failed: %s\n", message);
        exit(1);
    }
}

static Value tagged_text(const char *text, uint32_t tag) {
    Value value = {.tag = tag, .payload.handle = NULL};
    require(xiao_runtime_string_new(bytes(text), &value.payload.handle) == 0, "string setup");
    return value;
}

static int text_is(Value value, const char *text) {
    unsigned char buffer[64];
    size_t length;
    if (xiao_runtime_string_copy(value.payload.handle, (Buffer){buffer, sizeof(buffer)}, &length) != 0) {
        return 0;
    }
    return length == strlen(text) && memcmp(buffer, text, length) == 0;
}

static Snapshot pending(void) {
    Snapshot snapshot = {0};
    require(xiao_runtime_error_snapshot(&snapshot) == 0, "pending snapshot");
    return snapshot;
}

static void check_code(Snapshot snapshot, const char *code) {
    require(snapshot.code.len == strlen(code) &&
            memcmp(snapshot.code.ptr, code, snapshot.code.len) == 0, "pending code");
}

static void sentinel(void) {
    require(xiao_runtime_error_raise_type(bytes("Error"), bytes("B6_SENTINEL"),
                                         bytes("sentinel"), (Location){0}) == 0, "pending setup");
}

static int fatal(void) {
    Value value = {.tag = INT, .payload.integer = 73};
    int raised = xiao_runtime_error_raise_type(bytes("FatalError"), bytes(""), bytes(""), (Location){0});
    Snapshot before = pending();
    require(raised == 8 && before.class == 2 && before.exit_code == 4, "fatal status and class");
    require(xiao_runtime_error_take(&value) == 8, "fatal is not a value");
    Snapshot after = pending();
    check_code(after, "X07-FATAL-006");
    require(before.id == after.id && xiao_runtime_error_matches(bytes("Error")) == 0, "fatal routing");
    printf("fatal\t%d\t%.*s\t%" PRIu64 "\t%" PRIu64 "\t%d\n",
           raised, (int)after.code.len, (const char *)after.code.ptr, before.id, after.id, after.exit_code);
    xiao_runtime_error_report();
    return xiao_runtime_error_exit_code();
}

typedef enum { GOOD, NULL_ARGUMENT, UNKNOWN_TAG, NULL_HANDLE, WRONG_TYPE, PARAMETER,
               LEFT_ADMISSION, RIGHT_ADMISSION, ZERO_DENOMINATOR, RESULT_SCALE } Case;

static void divide(Case which, const char *name, int expected, const char *code) {
    const char *left = "1.00", *right = "2", *rounding = "half_even";
    int precision = 32768;
    if (which == PARAMETER) { precision = 0; rounding = "bad"; right = "0"; }
    if (which == LEFT_ADMISSION) { left = "1e100001"; right = "bad"; }
    if (which == RIGHT_ADMISSION) { left = "0"; right = "bad"; }
    if (which == ZERO_DENOMINATOR) { right = "0"; }
    if (which == RESULT_SCALE) { left = "1e100000"; right = "1e-100000"; }
    Value a = tagged_text(left, LFLOAT), b = tagged_text(right, LFLOAT);
    Value r = tagged_text(rounding, STR), p = {.tag = INT, .payload.integer = precision};
    Value out = tagged_text("old-out", STR);
    void *old_handle = out.payload.handle;
    void *weak = xiao_runtime_weak(old_handle);
    require(weak != NULL, "weak observation");
    Value bad = {.tag = which == UNKNOWN_TAG ? UNKNOWN : LFLOAT, .payload.handle = NULL};
    const Value *argument = &a;
    if (which == NULL_ARGUMENT) argument = NULL;
    if (which == UNKNOWN_TAG || which == NULL_HANDLE) argument = &bad;
    if (which == WRONG_TYPE) p.tag = FLOAT;
    xiao_runtime_error_clear();
    if (expected < 6) sentinel();
    Snapshot before = pending();
    int status = xiao_runtime_lfloat_divide(argument, &b, &p, &r, &out);
    Snapshot after = pending();
    require(status == expected, name);
    check_code(after, code);
    require(after.class == 1 && (expected == 6 ? before.id == 0 && after.id != 0 : before.id == after.id),
            "pending identity and class");
    void *upgraded = xiao_runtime_weak_upgrade(weak);
    if (expected == 0) {
        require(upgraded == NULL && out.tag == LFLOAT && text_is(out, "0.5"), "successful out replacement");
    } else {
        require(upgraded != NULL && out.tag == STR && out.payload.handle == old_handle && text_is(out, "old-out"),
                "failure preserved owned out");
        xiao_runtime_release(upgraded);
    }
    require(text_is(a, left) && text_is(b, right) && text_is(r, rounding), "borrowed inputs changed");
    printf("%s\t%d\t%s\t%" PRIu64 "\t%" PRIu64 "\t%s\towned\n", name, status, code,
           before.id, after.id, expected == 0 ? "0.5" : "old-out");
    xiao_runtime_weak_release(weak);
    xiao_runtime_value_release(&out);
    xiao_runtime_value_release(&r);
    xiao_runtime_value_release(&b);
    xiao_runtime_value_release(&a);
    xiao_runtime_error_clear();
}

int main(int argc, char **argv) {
    require(xiao_runtime_abi_version() == 1 && xiao_runtime_abi_minor_version() == 19, "Runtime ABI");
    if (argc == 2 && strcmp(argv[1], "fatal") == 0) return fatal();
    divide(GOOD, "divide-ok", 0, "B6_SENTINEL");
    divide(NULL_ARGUMENT, "divide-null", 1, "B6_SENTINEL");
    divide(UNKNOWN_TAG, "divide-label", 2, "B6_SENTINEL");
    divide(NULL_HANDLE, "divide-handle", 3, "B6_SENTINEL");
    divide(WRONG_TYPE, "divide-type", 6, "X06-RUNTIME-002");
    divide(PARAMETER, "divide-parameter", 6, "X06-RUNTIME-012");
    divide(LEFT_ADMISSION, "divide-left-admission", 6, "X06-RUNTIME-033");
    divide(RIGHT_ADMISSION, "divide-right-admission", 6, "X06-RUNTIME-012");
    divide(ZERO_DENOMINATOR, "divide-zero", 6, "X06-RUNTIME-013");
    divide(RESULT_SCALE, "divide-result-scale", 6, "X06-RUNTIME-033");
    return 0;
}
