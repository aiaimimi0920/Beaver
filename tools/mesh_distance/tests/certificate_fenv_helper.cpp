// Test-only explicit-build helper. Never imported by production code.
#include <cstdint>
#if !defined(__x86_64__) || !defined(__linux__)
#error This test helper supports x86_64 Linux only
#endif
#include <xmmintrin.h>
extern "C" uint32_t certificate_test_get_mxcsr() { return _mm_getcsr(); }
extern "C" void certificate_test_set_mxcsr(uint32_t value) { _mm_setcsr(value); }
