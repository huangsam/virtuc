use super::common::run_source;

#[test]
fn test_compile_and_run_simple_program() {
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int main() {
            return add(30, 12);
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_compile_and_run_control_flow() {
    let source = r#"
        int main() {
            int a = 10;
            if (a > 5) {
                return 1;
            } else {
                return 0;
            }
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(1));
}

#[test]
fn test_compile_and_run_with_printf() {
    let source = r#"
        extern int printf(string, ...);

        int main() {
            printf("Hello, World!\n");
            return 42;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("Hello, World!"));
}

#[test]
fn test_compile_and_run_with_include() {
    let source = r#"
        #include <stdio.h>

        int main() {
            printf("Hello from include!\n");
            return 7;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(7));
    assert!(res.stdout.contains("Hello from include!"));
}

#[test]
fn test_printf_with_integer() {
    let source = r#"
        extern int printf(string, ...);

        int main() {
            printf("Number: %d\n", 7);
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(0));
    assert!(res.stdout.contains("Number: 7"));
}

#[test]
fn test_printf_with_multiple_args_include() {
    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%s %d\n", "Hi", 10);
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(0));
    assert!(res.stdout.contains("Hi 10"));
}

#[test]
fn test_variadic_format_mix() {
    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%s %d %x\n", "val", 42, 255);
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(0));
    assert!(res.stdout.contains("val 42 ff") || res.stdout.contains("val 42 FF"));
}

#[test]
fn test_for_loop_sum() {
    let source = r#"
        int main() {
            int sum = 0;
            for (int i = 1; i <= 10; i = i + 1) {
                sum = sum + i;
            }
            return sum;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(55));
}

#[test]
fn test_for_loop_with_printf() {
    let source = r#"
        #include <stdio.h>

        int main() {
            for (int i = 0; i < 5; i = i + 1) {
                printf("%d ", i);
            }
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(0));
    assert_eq!(res.stdout.trim(), "0 1 2 3 4");
}

#[test]
fn test_modulo_operator() {
    let source = r#"
        int main() {
            int a = 47;
            int b = 10;
            int rem = a % b; // 7
            int even = 100 % 2; // 0
            int odd = 101 % 2; // 1
            return rem + even * 10 + odd * 35; // 7 + 0 + 35 = 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_unary_operators() {
    let source = r#"
        int main() {
            int x = 10;
            int neg = -x;       // -10
            int pos = -neg;     // 10
            int not_zero = !0;  // 1
            int not_pos = !x;   // 0
            int not_not = !(!x);// 1
            // 10 + 10 + 20*not_zero + 2*not_not = 10 + 10 + 20 + 2 = 42
            return pos + (-neg) + (20 * not_zero) + (2 * not_not);
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_while_loop() {
    let source = r#"
        int main() {
            int count = 0;
            int sum = 0;
            while (count < 10) {
                count = count + 1;
                sum = sum + count;
            }
            // sum of 1..10 = 55
            return sum;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(55));
}

#[test]
fn test_increment_decrement_and_compound_assign() {
    let source = r#"
        int main() {
            int sum = 0;
            // Test postfix in for loop update and += in body
            for (int i = 0; i < 5; i++) {
                sum += i; // 0 + 1 + 2 + 3 + 4 = 10
            }

            // Test prefix inc/dec
            ++sum; // 11
            --sum; // 10

            // Test -=, *=, /=, %=
            sum -= 2;  // 8
            sum *= 6;  // 48
            sum /= 2;  // 24
            sum %= 10; // 4

            // 4 * 10 + 2 = 42
            int res = sum * 10 + 2;
            return res;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_void_functions() {
    let source = r#"
        #include <stdio.h>

        void print_msg(string msg) {
            printf("%s\n", msg);
            return;
        }

        void no_op() {
            // implicit return void
        }

        int main() {
            print_msg("Void test executed");
            no_op();
            return 42;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("Void test executed"));
}

#[test]
fn test_stdlib_header() {
    let source = r#"
        #include <stdlib.h>

        int main() {
            int negative = -42;
            int positive = abs(negative);
            exit(positive);
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_math_header() {
    let source = r#"
        #include <stdio.h>
        #include <math.h>

        int main() {
            float s = sqrt(16.0);
            float p = pow(2.0, 3.0);
            printf("sqrt=%.1f pow=%.1f\n", s, p);
            return 42;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("sqrt=4.0 pow=8.0"));
}

#[test]
fn test_1d_arrays() {
    let source = r#"
        #include <stdio.h>

        int main() {
            int arr[5];
            for (int i = 0; i < 5; i++) {
                arr[i] = (i + 1) * 10;
            }

            // arr is now [10, 20, 30, 40, 50]
            arr[2] += 5; // arr[2] = 35
            arr[4]++;    // arr[4] = 51

            int sum = 0;
            for (int i = 0; i < 5; i++) {
                sum += arr[i];
            }
            // sum = 10 + 20 + 35 + 40 + 51 = 156
            printf("sum=%d\n", sum);
            return sum;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(156));
    assert!(res.stdout.contains("sum=156"));
}

#[test]
fn test_pointers_and_references() {
    let source = r#"
        #include <stdio.h>

        void swap(int* a, int* b) {
            int tmp = *a;
            *a = *b;
            *b = tmp;
        }

        int main() {
            // 1. Basic pointer and dereference assignment
            int val = 10;
            int* p = &val;
            *p = 25;

            // 2. Pass-by-pointer mutation
            int a = 1;
            int b = 2;
            swap(&a, &b);
            // now a == 2, b == 1

            // 3. Pointer to array decay, indexing, and pointer arithmetic
            int arr[3];
            arr[0] = 10;
            arr[1] = 20;
            arr[2] = 30;

            int* arr_ptr = arr;
            arr_ptr[1] = 5; // arr[1] becomes 5
            int elem2 = *(arr_ptr + 2); // 30

            // Pointer comparison
            int is_same = (arr_ptr == arr); // 1
            int is_diff = ((arr_ptr + 1) != arr_ptr); // 1

            // Result: val(25) + a(2) + b(1) + arr[1](5) + elem2(30) - 21 = 42
            int res = val + a + b + arr[1] + elem2 - 21;
            printf("val=%d a=%d b=%d arr1=%d elem2=%d same=%d diff=%d res=%d\n",
                   val, a, b, arr[1], elem2, is_same, is_diff, res);
            return res;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("val=25 a=2 b=1 arr1=5 elem2=30 same=1 diff=1 res=42"));
}

#[test]
fn test_float_operations_and_comparisons() {
    let source = r#"
        #include <stdio.h>

        float add(float a, float b) {
            return a + b;
        }

        float sub(float a, float b) {
            return a - b;
        }

        float mul(float a, float b) {
            return a * b;
        }

        float div(float a, float b) {
            return a / b;
        }

        int main() {
            float x = 10.5;
            float y = 2.5;

            float a = add(x, y); // 13.0
            float s = sub(x, y); // 8.0
            float m = mul(x, y); // 26.25
            float d = div(x, y); // 4.2
            float neg = -x;      // -10.5

            int cmp_lt = (x < y);    // 0
            int cmp_le = (x <= 10.5); // 1
            int cmp_gt = (x > y);    // 1
            int cmp_ge = (y >= 2.5);  // 1
            int cmp_eq = (x == 10.5); // 1
            int cmp_ne = (x != y);   // 1

            // Test float in if-condition
            int cond_result = 0;
            if (x > y) {
                cond_result = 42;
            }

            printf("a=%.1f s=%.1f m=%.2f d=%.1f neg=%.1f\n", a, s, m, d, neg);
            printf("cmp=%d%d%d%d%d%d cond=%d\n", cmp_lt, cmp_le, cmp_gt, cmp_ge, cmp_eq, cmp_ne, cond_result);
            return cond_result;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("a=13.0 s=8.0 m=26.25 d=4.2 neg=-10.5"));
    assert!(res.stdout.contains("cmp=011111 cond=42"));
}

#[test]
fn test_float_arrays_and_pointers() {
    let source = r#"
        #include <stdio.h>

        void scale(float* arr, int n, float factor) {
            for (int i = 0; i < n; i++) {
                arr[i] = arr[i] * factor;
            }
        }

        int main() {
            float vals[3];
            vals[0] = 1.5;
            vals[1] = 2.5;
            vals[2] = 3.5;

            // Pass array (decayed to pointer) to function
            scale(vals, 3, 2.0); // vals become [3.0, 5.0, 7.0]

            // Pointer to float
            float* p = &vals[1];
            *p = 10.0; // vals[1] becomes 10.0

            printf("v0=%.1f v1=%.1f v2=%.1f\n", vals[0], vals[1], vals[2]);
            return 42;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("v0=3.0 v1=10.0 v2=7.0"));
}

