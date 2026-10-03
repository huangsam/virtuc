use super::common::run_source;

#[test]
fn test_duplicate_includes() {
    let source = r#"
        #include <stdio.h>
        #include <stdio.h>

        int main() {
            printf("Duplicate include\n");
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(0));
    assert!(res.stdout.contains("Duplicate include"));
}

#[test]
fn test_printf_many_args() {
    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%d %d %d %d %d %d %d %d %d %d\n", 1,2,3,4,5,6,7,8,9,10);
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(0));
    assert!(res.stdout.contains("1 2 3 4 5 6 7 8 9 10"));
}

#[test]
fn test_block_comments() {
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

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_break_and_continue() {
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

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_short_circuit_logical_operators() {
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

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_omitted_for_loop_clauses() {
    let source = r#"
        int main() {
            int sum = 0;

            // 1. Omitted init
            int i = 0;
            for (; i < 3; i++) {
                sum += 1;
            }

            // 2. Omitted update
            int j = 0;
            for (; j < 3; ) {
                sum += 1;
                j++;
            }

            // 3. Omitted cond
            for (int k = 0; ; k++) {
                sum += 1;
                if (k == 2) {
                    break;
                }
            }

            // 4. All clauses omitted
            for (;;) {
                sum += 33;
                break;
            }

            // 3 + 3 + 3 + 33 = 42
            return sum;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_nested_loops_break_and_continue() {
    let source = r#"
        int main() {
            int outer_count = 0;
            int inner_sum = 0;

            for (int i = 0; i < 4; i++) {
                outer_count++;
                for (int j = 0; j < 10; j++) {
                    if (j == 3) {
                        break; // break inner loop only
                    }
                    if (j % 2 == 1) {
                        continue; // continue inner loop
                    }
                    inner_sum += 1;
                }
            }

            // Each outer iteration runs j=0 (sum+1), j=1 (continue), j=2 (sum+1), j=3 (break)
            // inner_sum = 2 per outer iteration * 4 = 8
            // outer_count = 4
            // 8 * 5 + 4 - 2 = 42
            return inner_sum * 5 + outer_count - 2;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_dereference_compound_and_prefix_inc_dec() {
    let source = r#"
        int main() {
            int val = 10;
            int* p = &val;

            // Pointer dereference compound assignment
            *p += 5;  // 15
            *p -= 3;  // 12
            *p *= 4;  // 48
            *p /= 2;  // 24
            *p %= 10; // 4

            // Prefix dereference inc/dec
            ++*p;     // 5
            --*p;     // 4

            // Array prefix inc/dec and postfix dec
            int arr[3];
            arr[0] = 10;
            arr[1] = 20;
            arr[2] = 30;

            ++arr[0]; // 11
            --arr[1]; // 19
            arr[2]--; // 29

            // 4 + 11 + 19 + 29 = 63; 63 - 21 = 42
            return (*p + arr[0] + arr[1] + arr[2]) - 21;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_loop_variable_reuse() {
    let source = r#"
        int main() {
            int sum = 0;
            for (int i = 0; i < 5; i++) {
                sum += i; // 0 + 1 + 2 + 3 + 4 = 10
            }
            // Re-declare and use i in a subsequent loop
            for (int i = 0; i < 5; i++) {
                sum += i; // 10 + 10 = 20
            }
            return sum + 22; // 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

