#include <iostream>
#include <lbd/fe/ast.h>
#include <lbd/fe/lexer.h>
#include <lbd/fe/parser.h>
#include <lbd/intp/interpreter.h>
#include <lbd/cmd.h>
#include <lbd/repl.h>
#include <string>
#include <vector>

const std::string &program_name = "lbd";

int main(const int argc, char **argv) {
    const auto &[filepath, show_help, repl, debug] = cmd::parse_args(argc, argv, program_name);
    if (show_help) {
        cmd::print_help(std::cout, argv[0]);
        return EXIT_SUCCESS;
    }
    if (repl) {
        repl::loop(debug);
    } else {
        // Lex
        auto lexer_v = fe::lexer::Lexer(*filepath, fe::lexer::FromFile{});
        const std::vector<fe::token::Token> tokens = lexer_v.lex_all();
        if (debug) {
            for (const fe::token::Token &token: tokens) {
                std::cout << token << std::endl;
            }
        }
        // Parse
        auto parser = fe::parser::Parser(tokens);
        if (debug) {
            std::cout << parser.program << std::endl;
        }
        // Interpret
        auto result = intp::interp::interpret(parser.program);
        return EXIT_SUCCESS;
    }
}
