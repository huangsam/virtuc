use super::common::{compile_source, run_source};

#[test]
fn test_basic_struct_fields() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        int main() {
            struct Point p;
            p.x = 15;
            p.y = 27;
            return p.x + p.y;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_pointer_and_arrow() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        void translate(struct Point* p, int dx, int dy) {
            p->x = p->x + dx;
            p->y = p->y + dy;
        }

        int main() {
            struct Point p;
            p.x = 10;
            p.y = 20;

            translate(&p, 5, 7);

            return p.x + p.y; // 15 + 27 = 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_compound_assign_and_inc_dec() {
    let source = r#"
        struct Counter {
            int val;
        };

        int main() {
            struct Counter c;
            c.val = 10;
            c.val += 5;   // 15
            c.val++;      // 16
            ++c.val;      // 17

            struct Counter* ptr = &c;
            ptr->val += 20; // 37
            ptr->val++;     // 38
            ++ptr->val;     // 39
            ptr->val -= 3;  // 36
            ptr->val--;     // 35
            --ptr->val;     // 34

            return c.val + 8; // 34 + 8 = 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_by_value_param_and_return() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        struct Point make_point(int a, int b) {
            struct Point p;
            p.x = a;
            p.y = b;
            return p;
        }

        int sum_point(struct Point p) {
            return p.x + p.y;
        }

        int main() {
            struct Point p = make_point(20, 22);
            return sum_point(p); // 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_copy_assignment() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        int main() {
            struct Point p1;
            p1.x = 10;
            p1.y = 20;

            struct Point p2 = p1;
            p2.x = 99; // should not affect p1

            struct Point p3;
            p3 = p1;
            p3.y = 88; // should not affect p1

            // p1.x + p1.y = 10 + 20 = 30
            // 30 + 12 = 42
            return p1.x + p1.y + 12;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_nested_structs() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        struct Rect {
            struct Point top_left;
            struct Point bottom_right;
        };

        int main() {
            struct Rect r;
            r.top_left.x = 5;
            r.top_left.y = 10;
            r.bottom_right.x = 15;
            r.bottom_right.y = 12;

            // 5 + 10 + 15 + 12 = 42
            return r.top_left.x + r.top_left.y + r.bottom_right.x + r.bottom_right.y;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_with_mixed_types_and_printf() {
    let source = r#"
        #include <stdio.h>

        struct Person {
            string name;
            int age;
            float score;
        };

        int main() {
            struct Person p;
            p.name = "Alice";
            p.age = 30;
            p.score = 98.5;

            printf("Person: %s, Age: %d, Score: %f\n", p.name, p.age, p.score);

            if (p.age == 30) {
                return 42;
            }
            return 1;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("Person: Alice"));
}

#[test]
fn test_struct_linked_list() {
    let source = r#"
        struct Node {
            int val;
            struct Node* next;
        };

        int main() {
            struct Node n1;
            struct Node n2;
            struct Node n3;

            n1.val = 10;
            n1.next = &n2;

            n2.val = 20;
            n2.next = &n3;

            n3.val = 12;

            int chained_sum = n1.val + n1.next->val + n1.next->next->val;

            int sum = 0;
            struct Node* curr = &n1;
            for (int i = 0; i < 3; i++) {
                sum += curr->val;
                curr = curr->next;
            }

            if (chained_sum == 42 && sum == 42) {
                return 42;
            }
            return 0;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_array_of_structs() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        int main() {
            struct Point pts[3];
            pts[0].x = 5;
            pts[0].y = 7;
            pts[1].x = 10;
            pts[1].y = 12;
            pts[2].x = 4;
            pts[2].y = 4;

            int sum = 0;
            for (int i = 0; i < 3; i++) {
                sum += pts[i].x + pts[i].y;
            }

            return sum; // (5+7) + (10+12) + (4+4) = 12 + 22 + 8 = 42
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_fixture_program() {
    let source = std::fs::read_to_string("tests/fixtures/structs.c").expect("read structs.c");
    let res = run_source(&source);
    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("Point: (15, 27)"));
}

#[test]
fn test_undefined_struct_type_fails() {
    let source = r#"
        int main() {
            struct NonExistent s;
            return 0;
        }
    "#;
    assert!(compile_source(source).is_err());
}

#[test]
fn test_access_undefined_field_fails() {
    let source = r#"
        struct Point {
            int x;
        };

        int main() {
            struct Point p;
            p.unknown = 10;
            return 0;
        }
    "#;
    assert!(compile_source(source).is_err());
}

#[test]
fn test_arrow_on_non_pointer_fails() {
    let source = r#"
        struct Point {
            int x;
        };

        int main() {
            struct Point p;
            p->x = 10;
            return 0;
        }
    "#;
    assert!(compile_source(source).is_err());
}

#[test]
fn test_field_type_mismatch_fails() {
    let source = r#"
        struct Point {
            int x;
        };

        int main() {
            struct Point p;
            p.x = 3.14;
            return 0;
        }
    "#;
    assert!(compile_source(source).is_err());
}

#[test]
fn test_duplicate_field_fails() {
    let source = r#"
        struct Point {
            int x;
            int x;
        };

        int main() {
            return 0;
        }
    "#;
    assert!(compile_source(source).is_err());
}

#[test]
fn test_struct_address_of_member_and_arrow() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        void add_five(int* val) {
            *val = *val + 5;
        }

        int main() {
            struct Point p;
            p.x = 10;
            p.y = 20;

            add_five(&p.x); // p.x becomes 15

            struct Point* ptr = &p;
            add_five(&ptr->y); // p.y becomes 25

            // 15 + 25 + 2 = 42
            return p.x + p.y + 2;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}

#[test]
fn test_struct_pointer_return_and_array_element() {
    let source = r#"
        struct Point {
            int x;
            int y;
        };

        struct Point* choose_point(struct Point* p1, struct Point* p2, int pick_second) {
            if (pick_second) {
                return p2;
            }
            return p1;
        }

        int main() {
            struct Point pts[2];
            pts[0].x = 10;
            pts[0].y = 12;
            pts[1].x = 20;
            pts[1].y = 22;

            struct Point* chosen = choose_point(&pts[0], &pts[1], 1);
            // chosen points to pts[1]: 20 + 22 = 42
            return chosen->x + chosen->y;
        }
    "#;

    let res = run_source(source);
    assert_eq!(res.exit_code, Some(42));
}
