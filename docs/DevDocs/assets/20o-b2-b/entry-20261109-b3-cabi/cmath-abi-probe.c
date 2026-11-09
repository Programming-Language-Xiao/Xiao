/* B3外部C调用验证；不经Xiao类型/IR/VM/LLVM。 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef union Payload { uint64_t raw; int64_t integer; double number; void *handle; } Payload;
typedef struct Value { uint32_t tag; Payload payload; } Value;
_Static_assert(sizeof(Value) == 16, "XiaoValue size");
_Static_assert(offsetof(Value, payload) == 8, "XiaoValue payload offset");

#define UNARY(name) extern int32_t xiao_runtime_cmath_##name(const Value *, Value *)
#define BINARY(name) extern int32_t xiao_runtime_cmath_##name(const Value *, const Value *, Value *)
UNARY(sqrt); UNARY(floor); UNARY(ceil); UNARY(round); UNARY(trunc); UNARY(abs);
BINARY(min); BINARY(max); BINARY(pow);
UNARY(sin); UNARY(cos); UNARY(tan); UNARY(exp); UNARY(ln); UNARY(log10);
extern void xiao_runtime_error_clear(void);

typedef int32_t (*Unary)(const Value *, Value *);
typedef int32_t (*Binary)(const Value *, const Value *, Value *);
typedef struct Entry {
    const char *name;
    Unary unary;
    Binary binary;
    double a, b, expected;
} Entry;
static const Entry entries[] = {
    {"sqrt", xiao_runtime_cmath_sqrt, NULL, 4.0, 0.0, 2.0},
    {"floor", xiao_runtime_cmath_floor, NULL, 1.5, 0.0, 1.0},
    {"ceil", xiao_runtime_cmath_ceil, NULL, 1.5, 0.0, 2.0},
    {"round", xiao_runtime_cmath_round, NULL, 2.5, 0.0, 2.0},
    {"trunc", xiao_runtime_cmath_trunc, NULL, -1.5, 0.0, -1.0},
    {"abs", xiao_runtime_cmath_abs, NULL, -0.0, 0.0, 0.0},
    {"min", NULL, xiao_runtime_cmath_min, -0.0, 0.0, -0.0},
    {"max", NULL, xiao_runtime_cmath_max, -0.0, 0.0, 0.0},
    {"pow", NULL, xiao_runtime_cmath_pow, 2.0, 3.0, 8.0},
    {"sin", xiao_runtime_cmath_sin, NULL, -0.0, 0.0, -0.0},
    {"cos", xiao_runtime_cmath_cos, NULL, 0.0, 0.0, 1.0},
    {"tan", xiao_runtime_cmath_tan, NULL, -0.0, 0.0, -0.0},
    {"exp", xiao_runtime_cmath_exp, NULL, 0.0, 0.0, 1.0},
    {"ln", xiao_runtime_cmath_ln, NULL, 1.0, 0.0, 0.0},
    {"log10", xiao_runtime_cmath_log10, NULL, 1.0, 0.0, 0.0}
};
static unsigned checks;
#define CHECK(condition) do { ++checks; if (!(condition)) { \
    fprintf(stderr, "C ABI check failed at line %d\n", __LINE__); return 1; } } while (0)

static Value number(double x) {
    Value result;
    memset(&result, 0, sizeof result);
    result.tag = 5;
    result.payload.number = x;
    return result;
}
static uint64_t bits(double x) {
    uint64_t result;
    memcpy(&result, &x, sizeof result);
    return result;
}
static int32_t call(const Entry *entry, const Value *a, const Value *b, Value *out) {
    return entry->unary ? entry->unary(a, out) : entry->binary(a, b, out);
}

int main(void) {
    for (size_t i = 0; i < sizeof entries / sizeof entries[0]; ++i) {
        const Entry *entry = &entries[i];
        Value a = number(entry->a), b = number(entry->b), out = number(-99.0);
        xiao_runtime_error_clear();
        CHECK(call(entry, &a, &b, &out) == 0);
        CHECK(out.tag == 5 && bits(out.payload.number) == bits(entry->expected));
        out = number(-99.0);
        Value saved = out;
        CHECK(call(entry, NULL, &b, &out) == 1);
        CHECK(memcmp(&out, &saved, sizeof out) == 0);
        CHECK(call(entry, &a, &b, NULL) == 1);
        if (entry->binary) {
            CHECK(call(entry, &a, NULL, &out) == 1);
            CHECK(memcmp(&out, &saved, sizeof out) == 0);
        }
        Value unknown = number(0.0);
        unknown.tag = 999;
        CHECK(call(entry, &unknown, &b, &out) == 2);
        CHECK(memcmp(&out, &saved, sizeof out) == 0);
        Value bad_handle = number(0.0);
        bad_handle.tag = 8;
        bad_handle.payload.handle = NULL;
        CHECK(call(entry, &bad_handle, &b, &out) == 3);
        CHECK(memcmp(&out, &saved, sizeof out) == 0);
        Value integer = number(0.0);
        integer.tag = 2;
        integer.payload.integer = 4;
        CHECK(call(entry, &integer, &b, &out) == 6);
        CHECK(memcmp(&out, &saved, sizeof out) == 0);
        xiao_runtime_error_clear();
        Value nonfinite = number(0.0);
        nonfinite.payload.raw = UINT64_C(0x7ff8000000000001);
        CHECK(call(entry, &nonfinite, &b, &out) == 6);
        CHECK(memcmp(&out, &saved, sizeof out) == 0);
        xiao_runtime_error_clear();
        CHECK(call(entry, &a, &b, &a) == 2);
        CHECK(bits(a.payload.number) == bits(entry->a));
    }
    printf("B3-CMATH-C-PROBE entries=15 checks=%u statuses=0,1,2,3,6 result=success\n", checks);
    return 0;
}
