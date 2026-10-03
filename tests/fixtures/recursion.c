int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

int fib(int n) {
    if (n <= 0) {
        return 0;
    }
    if (n == 1) {
        return 1;
    }
    return fib(n - 1) + fib(n - 2);
}

int main() {
    int f5 = factorial(5); // 120
    int fib7 = fib(7);     // 13
    // 120 - 13 = 107; 107 - 65 = 42
    return (f5 - fib7) - 65;
}
