use super::common::run_source;

#[test]
fn test_coin_change_dp() {
    let source = include_str!("../fixtures/coin_change.c");
    let res = run_source(source);

    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("min_coins(11) = 3"));
}

#[test]
fn test_quicksort_in_place() {
    let source = include_str!("../fixtures/quicksort.c");
    let res = run_source(source);

    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("sorted: 11 12 22 25 64 90"));
}

#[test]
fn test_binary_search() {
    let source = include_str!("../fixtures/binary_search.c");
    let res = run_source(source);

    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("found=5 missing=-1"));
}

#[test]
fn test_matrix_multiplication() {
    let source = include_str!("../fixtures/matrix_mul.c");
    let res = run_source(source);

    assert_eq!(res.exit_code, Some(42));
    assert!(res.stdout.contains("C = [[31, 19], [85, 55]]"));
}
