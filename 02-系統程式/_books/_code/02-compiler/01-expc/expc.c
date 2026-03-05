#include <stdio.h>
#include <ctype.h>
#include <stdlib.h>

// Global variable to track the current character position in the input string
const char *p;

// Preview the current character without moving the pointer
char peek() { return *p; }

// Read the current character and move the pointer to the next character
char get() { return *p++; }

// Function declarations (corresponding to EBNF grammar rules)
double expression();
double term();
double factor();
double number();

// 1. expression = term , { ( "+" | "-" ) , term } ;
double expression() {
    double result = term();
    while (peek() == '+' || peek() == '-') {
        if (get() == '+') result += term();
        else result -= term();
    }
    return result;
}

// 2. term = factor , { ( "*" | "/" ) , factor } ;
double term() {
    double result = factor();
    while (peek() == '*' || peek() == '/') {
        if (get() == '*') result *= factor();
        else result /= factor();
    }
    return result;
}

// 3. factor = number | "(" , expression , ")" ;
double factor() {
    if (peek() == '(') {
        get(); // 跳過 '('
        double result = expression();
        get(); // 跳過 ')'
        return result;
    }
    return number();
}

// 4. number = digit , { digit } ;
double number() {
    double result = 0;
    while (isdigit(peek())) {
        result = result * 10 + (get() - '0');
    }
    return result;
}

int main() {
    char input[100];
    printf("Please enter a mathematical expression (e.g., 3+5*(2-1)): ");
    scanf("%s", input);
    
    p = input;
    printf("Result: %.2f\n", expression());
    
    return 0;
}