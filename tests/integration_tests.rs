use std::process::Command;
use tempfile::TempDir;
use virtuc::compile;

#[test]
fn test_compile_and_run_simple_program() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_prog");

    // Source code
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int main() {
            return add(30, 12);
        }
    "#;

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    // Check exit code (30 + 12 = 42)
    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_compile_and_run_control_flow() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_flow");

    // Source code
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

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    // Check exit code
    assert_eq!(status.code(), Some(1));
}

#[test]
fn test_compile_and_run_with_printf() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf");

    // Source code
    let source = r#"
        extern int printf(string, ...);

        int main() {
            printf("Hello, World!\n");
            return 42;
        }
    "#;

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable and capture output
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    // Check exit code
    assert_eq!(output.status.code(), Some(42));

    // Check stdout contains "Hello, World!"
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, World!"));
}

#[test]
fn test_compile_and_run_with_include() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_include");

    // Source code using include
    let source = r#"
        #include <stdio.h>

        int main() {
            printf("Hello from include!\n");
            return 7;
        }
    "#;

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable and capture output
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    // Check exit code
    assert_eq!(output.status.code(), Some(7));

    // Check stdout contains "Hello from include!"
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from include!"));
}

#[test]
fn test_printf_with_integer() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf_int");

    let source = r#"
        extern int printf(string, ...);

        int main() {
            printf("Number: %d\n", 7);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");

    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Number: 7"));
}

#[test]
fn test_printf_with_multiple_args_include() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf_multi");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%s %d\n", "Hi", 10);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");

    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hi 10"));
}

#[test]
fn test_non_variadic_wrong_arity() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_wrong_arity");

    let source = r#"
        extern int foo(int);

        int main() {
            foo();
            return 0;
        }
    "#;

    assert!(compile(source, &output_path).is_err());
}

#[test]
fn test_printf_without_declaration_fails() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_no_decl");

    let source = r#"
        int main() {
            printf("No decl\n");
            return 0;
        }
    "#;

    assert!(compile(source, &output_path).is_err());
}

#[test]
fn test_duplicate_includes() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_dup_include");

    let source = r#"
        #include <stdio.h>
        #include <stdio.h>

        int main() {
            printf("Duplicate include\n");
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Duplicate include"));
}

#[test]
fn test_variadic_format_mix() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_variadic_mix");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%s %d %x\n", "val", 42, 255);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("val 42 ff") || stdout.contains("val 42 FF"));
}

#[test]
fn test_printf_many_args() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf_many");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%d %d %d %d %d %d %d %d %d %d\n", 1,2,3,4,5,6,7,8,9,10);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 2 3 4 5 6 7 8 9 10"));
}

#[test]
fn test_for_loop_sum() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_for_loop");

    // Test for loop that sums numbers 1 to 10
    let source = r#"
        int main() {
            int sum = 0;
            for (int i = 1; i <= 10; i = i + 1) {
                sum = sum + i;
            }
            return sum;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    // Sum of 1 to 10 is 55
    assert_eq!(status.code(), Some(55));
}

#[test]
fn test_for_loop_with_printf() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_for_printf");

    let source = r#"
        #include <stdio.h>

        int main() {
            for (int i = 0; i < 5; i = i + 1) {
                printf("%d ", i);
            }
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "0 1 2 3 4");
}

#[test]
fn test_block_comments() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_block_comments");

    let source = r#"
        /*
         * Multi-line comment header
         */
        #include <stdio.h>

        /* Function comment */
        int main() {
            int /* inline */ x = 100;
            /* Comment inside body
               spanning multiple lines */
            return x /* inline 2 */ - 58;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_modulo_operator() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_modulo");

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

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_unary_operators() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_unary");

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

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_while_loop() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_while");

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

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(55));
}

#[test]
fn test_increment_decrement_and_compound_assign() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_inc_dec");

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

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_void_functions() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_void");

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

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(42));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Void test executed"));
}

#[test]
fn test_break_and_continue() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_break_cont");

    let source = r#"
        int main() {
            int break_sum = 0;
            int i = 0;
            // Test break in while loop
            while (1) {
                if (i >= 10) {
                    break;
                }
                break_sum += i;
                i++;
            }
            // break_sum = 0 + 1 + ... + 9 = 45

            // Test continue in for loop: sum only odd numbers between 0 and 10
            // odd numbers: 1, 3, 5, 7, 9 -> sum = 25
            int odd_sum = 0;
            for (int j = 0; j < 10; j++) {
                if (j % 2 == 0) {
                    continue;
                }
                odd_sum += j;
            }

            // 45 - 25 = 20; 20 + 22 = 42
            return (break_sum - odd_sum) + 22;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_short_circuit_logical_operators() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_logical");

    let source = r#"
        int main() {
            int zero = 0;
            int one = 1;

            // Test short-circuit in &&: if not short-circuited, 10 / 0 would trap
            int and_res = 0;
            if (zero != 0 && (10 / zero) > 1) {
                and_res = 999;
            } else {
                and_res = 20;
            }

            // Test short-circuit in ||: if not short-circuited, 10 / (1 - 1) would trap
            int or_res = 0;
            if (one == 1 || (10 / (one - 1)) > 1) {
                or_res = 22;
            } else {
                or_res = 999;
            }

            // 20 + 22 = 42
            return and_res + or_res;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_stdlib_header() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_stdlib");

    let source = r#"
        #include <stdlib.h>

        int main() {
            int negative = -42;
            int positive = abs(negative);
            exit(positive);
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_math_header() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_math");

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

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(42));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sqrt=4.0 pow=8.0"));
}

#[test]
fn test_1d_arrays() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_arrays");

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

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(156));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sum=156"));
}

#[test]
fn test_pointers_and_references() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_pointers");

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

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(42));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("val=25 a=2 b=1 arr1=5 elem2=30 same=1 diff=1 res=42"));
}
