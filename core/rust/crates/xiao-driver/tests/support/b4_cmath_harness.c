/* B4-3测试宿主：执行真实lowering函数并逐IEEE位模式/错误码比对。
 * test_exit只替换测试模块的exit边，以setjmp观察静态错误；独立语言用例另测真实退出。
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <setjmp.h>
typedef struct { uint32_t tag; uint64_t payload; } Value;
typedef struct { const unsigned char *ptr; size_t len; } Bytes;
extern void xiao_runtime_error_clear(void);
extern uint32_t xiao_runtime_error_class(void);
extern int xiao_runtime_error_take(Value *out);
extern int xiao_runtime_error_code(const Value *value, Bytes *out);
extern void xiao_runtime_value_release(Value *value);
static jmp_buf failure;
static int observed_exit;
void test_exit(int code) { observed_exit = code; longjmp(failure, 1); }
void test_report(void) { }
/*DECLARATIONS*/
typedef struct { int function; uint64_t a,b,expected; int error; } Case;
static const Case cases[] = {
/*CASE_DATA*/
};
static uint64_t invoke(int which, uint64_t a_bits, uint64_t b_bits) {
    double a,b,result;
    memcpy(&a,&a_bits,8); memcpy(&b,&b_bits,8);
    Value av={5,a_bits}, bv={5,b_bits}, out={0,0};
    switch(which) {
/*CALLS*/
    default: return 0;
    }
/*RESULT*/
}
static int error_code_is(const char *expected) {
    Value error={0,0}; Bytes code={0,0};
    if(xiao_runtime_error_take(&error)!=0) return 0;
    int valid=xiao_runtime_error_code(&error,&code)==0 && code.len==strlen(expected) && memcmp(code.ptr,expected,code.len)==0;
    xiao_runtime_value_release(&error); return valid;
}
static int error_is_009(void) { return error_code_is("X06-RUNTIME-009"); }
int main(void) {
    unsigned passed=0,errors=0;
    for(size_t i=0;i<sizeof(cases)/sizeof(cases[0]);++i) {
        const Case *c=&cases[i]; xiao_runtime_error_clear(); observed_exit=0;
        uint64_t bits=0;
        if(setjmp(failure)==0) bits=invoke(c->function,c->a,c->b);
        int error=xiao_runtime_error_class()!=0;
        if(error!=c->error || (!error && bits!=c->expected) || (error && !error_is_009())) {
            fprintf(stderr,"case=%zu fn=%d bits=%016llx expected=%016llx error=%d expected_error=%d exit=%d\n",
                i,c->function,(unsigned long long)bits,(unsigned long long)c->expected,error,c->error,observed_exit);
            return 1;
        }
        ++passed; errors+=error;
    }
/*EXTRA_CHECKS*/
    printf("B4-3-NATIVE cases=%u error_cases=%u pow_inputs=1075 bit_or_code=exact\n",passed,errors);
    return 0;
}
