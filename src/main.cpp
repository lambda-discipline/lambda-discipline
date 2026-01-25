#include <iostream>
#include <string>
#include <vector>
#include <lbd/cmd.h>
#include <lbd/docs.h>
#include <lbd/repl.h>
#include <lbd/frontend/lexer.h>
#include <lbd/frontend/parser.h>
#include <lbd/runtime/interpreter.h>

const std::string &programName = "lbd";

int main(const int argc, char **argv) {
    const auto &[filepath, show_help, repl, debug, generateDocs] = cmd::parseArguments(argc, argv, programName);
    if (show_help) {
        cmd::printHelp(std::cout, argv[0]);
        return EXIT_SUCCESS;
    }
    if (generateDocs) {
        docs::dumpDocs(std::cout);
        return EXIT_SUCCESS;
    }
    if (repl) {
        repl::loop(debug);
    } else {
        // Lex
        auto lexerValue = frontend::Lexer::fromFile(*filepath);
        const std::vector<frontend::token::Token> tokens = lexerValue.lex();
        if (debug) {
            for (const frontend::token::Token &token: tokens) {
                std::cout << token << std::endl;
            }
        }
        // Parse
        auto program = frontend::Parser(tokens).parse();
        if (debug) {
            std::cout << program << std::endl;
        }
        // Interpret
        auto result = runtime::interpret(program);
        return EXIT_SUCCESS;
    }
}
