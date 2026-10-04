#include <stdio.h>

struct Point {
    int x;
    int y;
};

struct Rect {
    struct Point top_left;
    struct Point bottom_right;
};

void move_point(struct Point* p, int dx, int dy) {
    p->x += dx;
    p->y += dy;
}

int point_sum(struct Point p) {
    return p.x + p.y;
}

int main() {
    struct Point p;
    p.x = 10;
    p.y = 20;

    move_point(&p, 5, 7);

    struct Rect r;
    r.top_left.x = p.x;
    r.top_left.y = p.y;
    r.bottom_right.x = 50;
    r.bottom_right.y = 60;

    int sum = point_sum(p) + r.bottom_right.x; // (15 + 27) + 50 = 92
    printf("Point: (%d, %d), Rect BR: (%d, %d), Sum: %d\n",
           p.x, p.y, r.bottom_right.x, r.bottom_right.y, sum);

    if (p.x == 15 && p.y == 27 && sum == 92) {
        return 42;
    }
    return 1;
}
