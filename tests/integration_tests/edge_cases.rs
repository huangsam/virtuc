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
    let source = include_str!("../fixtures/for_loops.c");
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

#[test]
fn test_2d_array_prefix_postfix_and_compound() {
    let source = r#"
        int main() {
            int mat[2][2];
            mat[0][0] = 10;
            mat[0][1] = 20;
            mat[1][0] = 30;
            mat[1][1] = 40;

            ++mat[0][0]; // 11
            --mat[0][1]; // 19
            mat[1][0]--; // 29
            mat[1][1]++; // 41

            mat[0][0] += 5; // 16
            mat[0][1] -= 9; // 10
            mat[1][0] *= 2; // 58
            mat[1][1] /= 2; // 20

            // 16 + 10 + 58 + 20 = 104; 104 - 62 = 42
            return (mat[0][0] + mat[0][1] + mat[1][0] + mat[1][1]) - 62;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_float_2d_array() {
    let source = r#"
        #include <stdio.h>

        int main() {
            float mat[2][2];
            mat[0][0] = 1.5;
            mat[0][1] = 2.5;
            mat[1][0] = 3.5;
            mat[1][1] = 4.5;

            float sum = 0.0;
            for (int i = 0; i < 2; i++) {
                for (int j = 0; j < 2; j++) {
                    sum = sum + mat[i][j];
                }
            }
            // sum = 1.5 + 2.5 + 3.5 + 4.5 = 12.0
            printf("sum=%.1f\n", sum);
            if (sum == 12.0) {
                return 42;
            }
            return 1;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("sum=12.0"));
}

#[test]
fn test_2d_array_whole_decay_to_pointer() {
    let source = r#"
        void fill_buffer(int* p, int n) {
            for (int i = 0; i < n; i++) {
                p[i] = (i + 1) * 2;
            }
        }

        int main() {
            int mat[2][3];
            fill_buffer(mat, 6);
            // mat[0] = [2, 4, 6]
            // mat[1] = [8, 10, 12]
            return mat[1][2] + 30; // 12 + 30 = 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_nested_dynamic_index_expressions() {
    let source = r#"
        int main() {
            int mat[3][3];
            mat[0][0] = 1;
            mat[0][1] = 2;
            mat[1][2] = 42;

            // Computed indices: mat[mat[0][0]][mat[0][1]] -> mat[1][2] -> 42
            return mat[mat[0][0]][mat[0][1]];
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_3d_array() {
    let source = r#"
        int main() {
            int cube[2][3][4];
            cube[1][2][3] = 42;
            return cube[1][2][3];
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}
