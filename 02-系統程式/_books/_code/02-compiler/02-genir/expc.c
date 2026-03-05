#include <stdio.h>
#include <ctype.h>

const char *p;
int temp_count = 0; // used to generate t1, t2, t3, ...

// Generate a new temporary register number
int new_temp() {
    return ++temp_count;
}

// Forward declarations
int expression();
int term();
int factor();

// Handle numbers: directly print assignment instructions and return register 
int get_number() {
    int val = 0;
    while (isdigit(*p)) {
        val = val * 10 + (*p++ - '0');
    }
    int t = new_temp();
    printf("t%d = %d\n", t, val);
    return t;
}

// factor = number | "(" expression ")"
int factor() {
    if (*p == '(') {
        p++; // skip (
        int t = expression();
        p++; // skip )
        return t;
    }
    return get_number();
}

// term = factor { (*|/) factor }
int term() {
    int left = factor();
    while (*p == '*' || *p == '/') {
        char op = *p++;
        int right = factor();
        int target = new_temp();
        printf("t%d = t%d %c t%d\n", target, left, op, right);
        left = target;
    }
    return left;
}

// expression = term { (+|-) term }
int expression() {
    int left = term();
    while (*p == '+' || *p == '-') {
        char op = *p++;
        int right = term();
        int target = new_temp();
        // Print the three-address instruction code for multiplication/division
        printf("t%d = t%d %c t%d\n", target, left, op, right);
        left = target;
    }
    return left;
}

int main() {
    char input[100];
    printf("Please enter a mathematical expression (e.g., 3+5*(2-1)): ");
    scanf("%s", input);
    p = input;

    printf("\n--- Generated three-address code (3AC) ---\n");
    int final_t = expression();
    printf("Result is in t%d\n", final_t);
    
    return 0;
}