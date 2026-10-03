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
