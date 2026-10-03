#include <stdio.h>

int min_coins(int* coins, int num_coins, int amount) {
    int dp[100];
    dp[0] = 0;
    for (int i = 1; i <= amount; i++) {
        dp[i] = 999999;
    }

    for (int i = 1; i <= amount; i++) {
        for (int j = 0; j < num_coins; j++) {
            if (i >= coins[j]) {
                int rem = i - coins[j];
                if (dp[rem] + 1 < dp[i]) {
                    dp[i] = dp[rem] + 1;
                }
            }
        }
    }

    return dp[amount];
}

int main() {
    int coins[3];
    coins[0] = 1;
    coins[1] = 2;
    coins[2] = 5;

    // 11 = 5 + 5 + 1 -> min 3 coins
    int res = min_coins(coins, 3, 11);
    printf("min_coins(11) = %d\n", res);

    // 3 * 14 = 42
    return res * 14;
}
