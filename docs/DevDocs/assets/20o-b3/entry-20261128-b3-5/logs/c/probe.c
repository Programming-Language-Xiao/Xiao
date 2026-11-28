/* 直接链接发布 Runtime 的 B3-5C 探针。 */
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif

typedef struct {
    const uint8_t *ptr;
    size_t len;
} XiaoAbiBytes;

typedef struct {
    uint8_t *ptr;
    size_t capacity;
} XiaoAbiMutBytes;

typedef struct {
    uint32_t tag;
    union {
        uint64_t raw;
        void *handle;
    } payload;
} XiaoValue;

typedef struct {
    uint32_t class_tag, kind;
    uint64_t error_id;
    XiaoAbiBytes code, message_id;
    struct {
        uint64_t start, end;
        uint8_t present, reserved[7];
    } location;
    int32_t exit_code;
    uint32_t stack_depth, param_count;
} XiaoAbiErrorSnapshot;

_Static_assert(sizeof(XiaoValue) == 16, "XiaoValue layout");
_Static_assert(offsetof(XiaoValue, payload) == 8, "XiaoValue payload offset");
_Static_assert(sizeof(XiaoAbiErrorSnapshot) == 88, "error snapshot layout");

extern int32_t xiao_runtime_log_write(const XiaoValue *, const XiaoValue *,
                                     const XiaoValue *, uint64_t);
extern int32_t xiao_runtime_string_new(XiaoAbiBytes, void **);
extern int32_t xiao_runtime_string_copy(void *, XiaoAbiMutBytes, size_t *);
extern void xiao_runtime_value_release(XiaoValue *);
extern void xiao_runtime_error_clear(void);
extern int32_t xiao_runtime_error_snapshot(XiaoAbiErrorSnapshot *);
extern void xiao_runtime_release_trace_begin(void);
extern void xiao_runtime_release_trace_flush(void);
extern int32_t xiao_runtime_env_run_injected(int32_t (*)(void), const uint8_t *,
                                            size_t, uint64_t);

static const uint64_t both = UINT64_C(0x22);
static const char *expected_output_error;

static XiaoValue string_bytes(const void *text, size_t len) {
    XiaoValue value = { .tag = 8 };
    XiaoAbiBytes view = { text, len };
    assert(xiao_runtime_string_new(view, &value.payload.handle) == 0);
    return value;
}

static XiaoValue string(const char *text) {
    return string_bytes(text, strlen(text));
}

static void pending(uint32_t class_tag, const char *code, int32_t exit_code) {
    XiaoAbiErrorSnapshot error = {0};
    assert(xiao_runtime_error_snapshot(&error) == 0);
    assert(error.class_tag == class_tag && error.exit_code == exit_code);
    if (code != NULL) {
        assert(error.code.len == strlen(code));
        assert(memcmp(error.code.ptr, code, error.code.len) == 0);
    }
    xiao_runtime_error_clear();
}

static void release(XiaoValue *value) {
    xiao_runtime_value_release(value);
    assert(value->tag == 0 && value->payload.raw == 0);
}

static int32_t verify_in_run(void) {
    XiaoValue level = string("info");
    XiaoValue message = string("C ABI \xe6\xb5\x8b\xe8\xaf\x95");
    XiaoValue target = string("stdout");
    if (expected_output_error != NULL) {
        assert(xiao_runtime_log_write(&level, &message, &target, both) == 6);
        pending(1, expected_output_error, 3);
    } else {
        XiaoValue unknown = { .tag = 255 };
        XiaoValue bad_shape = { .tag = 8 };
        XiaoValue wrong_type = { .tag = 2, .payload.raw = 12 };
        assert(xiao_runtime_log_write(NULL, &message, &target, both) == 1);
        pending(1, "X06-RUNTIME-012", 3);
        assert(xiao_runtime_log_write(&level, &bad_shape, &target, both) == 1);
        pending(1, "X06-RUNTIME-012", 3);
        assert(xiao_runtime_log_write(&level, &message, &unknown, both) == 2);
        pending(1, "X06-RUNTIME-012", 3);
        assert(xiao_runtime_log_write(&level, &message, &target, both | 0x100) == 2);
        pending(1, "X06-RUNTIME-012", 3);
        assert(xiao_runtime_log_write(&wrong_type, &message, &target, 0) == 6);
        pending(1, "X06-RUNTIME-002", 3);
        for (uint64_t mask = 2; mask <= 0x20; mask <<= 4) {
            assert(xiao_runtime_log_write(&level, &message, &target, mask) == 6);
            pending(1, "X06-RUNTIME-026", 3);
        }
        assert(xiao_runtime_log_write(&target, &message, &level, both) == 6);
        pending(1, "X06-RUNTIME-012", 3);
        assert(xiao_runtime_log_write(&level, &message, &wrong_type, both) == 6);
        pending(1, "X06-RUNTIME-002", 3);
        XiaoValue wrong_target = string("stderr");
        assert(xiao_runtime_log_write(&level, &message, &wrong_target, both) == 6);
        pending(1, "X06-RUNTIME-012", 3);
        release(&wrong_target);
        const char *invalid[] = {"\r", "\n", "\0"};
        for (size_t i = 0; i < sizeof(invalid) / sizeof(invalid[0]); ++i) {
            XiaoValue invalid_message = string_bytes(invalid[i], 1);
            assert(xiao_runtime_log_write(&level, &invalid_message, &target, both) == 6);
            pending(1, "X06-RUNTIME-012", 3);
            release(&invalid_message);
        }
        char boundary[4097];
        memset(boundary, 'a', sizeof(boundary));
        XiaoValue too_long = string_bytes(boundary, sizeof(boundary));
        assert(xiao_runtime_log_write(&level, &too_long, &target, both) == 6);
        pending(1, "LOG_LIMIT_EXCEEDED", 3);
        release(&too_long);
        XiaoValue full = string_bytes(boundary, 4096);
        assert(xiao_runtime_log_write(&level, &full, &target, both) == 0);
        pending(0, NULL, 0);
        release(&full);

        const uint8_t invalid_utf8[] = { 0xc0, 0xaf };
        XiaoAbiBytes bytes = { invalid_utf8, sizeof(invalid_utf8) };
        void *handle = NULL;
        assert(xiao_runtime_string_new(bytes, &handle) == 4 && handle == NULL);
        pending(0, NULL, 0);
        void *old_message = message.payload.handle;
        assert(xiao_runtime_log_write(&level, &message, &target, both) == 0);
        pending(0, NULL, 0);
        assert(xiao_runtime_log_write(&level, &level, &target, both) == 2);
        pending(1, "X06-RUNTIME-012", 3);
        assert(message.tag == 8 && message.payload.handle == old_message);
        uint8_t copy[32];
        size_t copied = 0;
        assert(xiao_runtime_string_copy(old_message, (XiaoAbiMutBytes){copy, sizeof(copy)},
                                        &copied) == 0);
        assert(copied == 12 && memcmp(copy, "C ABI \xe6\xb5\x8b\xe8\xaf\x95", copied) == 0);
    }
    release(&target);
    release(&message);
    release(&level);
    return 0;
}

int main(int argc, char **argv) {
    xiao_runtime_release_trace_begin();
    if (argc == 2 && strcmp(argv[1], "closed") == 0) {
        expected_output_error = "LOG_OUTPUT_FAILED";
#ifdef _WIN32
        assert(SetStdHandle(STD_OUTPUT_HANDLE, NULL));
#else
        assert(close(STDOUT_FILENO) == 0);
#endif
    }
#ifdef _WIN32
    if (argc == 2 && strcmp(argv[1], "saturated") == 0) {
        expected_output_error = "LOG_TIMEOUT";
        HANDLE old = GetStdHandle(STD_OUTPUT_HANDLE), reader, writer;
        assert(CreatePipe(&reader, &writer, NULL, 4096));
        char bytes[4096];
        memset(bytes, 'A', sizeof(bytes));
        DWORD count = 0;
        assert(WriteFile(writer, bytes, sizeof(bytes), &count, NULL));
        assert(count == sizeof(bytes));
        assert(SetStdHandle(STD_OUTPUT_HANDLE, writer));
        ULONGLONG start = GetTickCount64();
        assert(xiao_runtime_env_run_injected(verify_in_run, NULL, 0, both) == 0);
        assert(GetTickCount64() - start >= 9900);
        assert(SetStdHandle(STD_OUTPUT_HANDLE, old));
        assert(PeekNamedPipe(reader, NULL, 0, NULL, &count, NULL));
        assert(count == sizeof(bytes));
        assert(CloseHandle(writer) && CloseHandle(reader));
        xiao_runtime_release_trace_flush();
        fprintf(stderr, "C ABI: LOG_TIMEOUT, pipe retained 4096 original bytes\n");
        return 0;
    }
#endif
    if (argc == 2 && strcmp(argv[1], "broken") == 0) {
        /* 运行者关闭管道读端后才启动探针。 */
        expected_output_error = "LOG_OUTPUT_FAILED";
    }
    assert(xiao_runtime_env_run_injected(verify_in_run, NULL, 0, both) == 0);
    if (expected_output_error == NULL) {
        XiaoValue level = string("info");
        XiaoValue message = string("unbound");
        XiaoValue target = string("stdout");
        assert(xiao_runtime_log_write(&level, &message, &target, both) == 8);
        pending(2, NULL, 4);
        release(&target);
        release(&message);
        release(&level);
    }
    xiao_runtime_release_trace_flush();
    fprintf(stderr, "%s\n", expected_output_error != NULL
        ? expected_output_error : "C ABI: 0/1/2/6/8, pending, borrowed slots and release");
    return 0;
}
