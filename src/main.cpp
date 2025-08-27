#include <iostream>
#include <lbd/fe/ast.h>
#include <lbd/fe/lexer.h>
#include <lbd/fe/parser.h>
#include <string>
#include <vector>

int main() {
    const std::string filepath = "../examples/std.lbd";
    auto lexer_v = fe::lexer::Lexer(filepath);

    const std::vector<fe::token::Token> tokens = lexer_v.lex_all();
    for (const fe::token::Token &token: tokens) {
        std::cout << token << std::endl;
    }

    const auto parser = fe::parser::Parser(tokens);
    std::cout << parser.program << std::endl;

    return 0;
}
