#include <stdio.h>
#include <ctype.h>

const char *p;
int temp_count = 0;

int new_temp() { return ++temp_count; }

// Forward declarations
int expression();
int term();
int factor();

// Handle numbers or variables (Identifier)
int get_atom() {
    if (isdigit(*p)) {
        int val = 0;
        while (isdigit(*p)) val = val * 10 + (*p++ - '0');
        int t = new_temp();
        printf("t%d = %d\n", t, val);
        return t;
    } else if (isalpha(*p)) {
        // Assume this is a variable, directly return a token representing it (or print it)
        char var_name = *p++;
        int t = new_temp();
        printf("t%d = load %c\n", t, var_name); // Simulate loading a variable from memory 
        return t;
    }
    return 0;
}

int factor() {
    if (*p == '(') {
        p++; int t = expression(); p++; return t;
    }
    return get_atom();
}

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

int expression() {
    int left = term();
    while (*p == '+' || *p == '-') {
        char op = *p++;
        int right = term();
        int target = new_temp();
        printf("t%d = t%d %c t%d\n", target, left, op, right);
        left = target;
    }
    return left;
}

// New: Handle assignment statements (assignment = id "=" expression)
void assignment() {
    if (isalpha(*p) && *(p+1) == '=') {
        char var_name = *p;
        p += 2; // Skip 'x='
        int result_t = expression();
        printf("store t%d into %c\n", result_t, var_name);
    } else {
        expression(); // If it's not an assignment, treat it as a normal expression
    }
}

int main() {
    char input[100];
    printf("Please enter an assignment statement (e.g., x=3+5*(2-y)): ");
    scanf("%s", input);
    p = input;

    printf("\n--- Generated Intermediate Code ---\n");
    assignment();
    
    return 0;
}