#include <iostream>
#include <lbd/repl.h>

#include "lbd/fe/lexer.h"
#include "lbd/fe/parser.h"
#include "lbd/intp/interpreter.h"

namespace repl {
    static bool force_on_env_dump = false;
    static bool debug_enabled = false;

    static void help() {
        std::cout
                << "available REPL commands:\n"
                << "  general:\n"
                << "    :q, :quit, :exit      Exit the REPL\n"
                << "    :c, :clear, :cls      Clear the screen\n"
                << "    :h, :help, :?         Show this help message\n"
                << "    :r, :reload           Reset environment (forget all bindings)\n"
                << "    :d, :debug            Toggle debug mode\n"
                << "  inspection:\n"
                << "    :e, :env              Show all variables in current environment\n"
                << "    :force                Toggle forcing evaluation when dumping env\n"
                << "current options:\n"
                << "    debug                 [" << (debug_enabled ? "on " : "off") << "]   (use :debug to toggle)\n"
                << "    force-on-env-dump     [" << (force_on_env_dump ? "on " : "off") << "]   (use :force to toggle)"
                << std::endl;
    }

    void loop(const bool debug) {
        debug_enabled = debug;
        std::cout << "lambda-discipline REPL.\ntype :quit to exit." << std::endl;
        std::string line;
        std::optional<std::shared_ptr<intp::interp::Env> > shared_global_env = std::nullopt;
        while (true) {
            std::cout << "lbd> ";
            if (!std::getline(std::cin, line)) {
                break; // EOF (C-d / C-z)
            }
            if (line == ":q" || line == ":quit" || line == ":exit") {
                std::cout << "exiting REPL" << std::endl;
                break;
            }
            if (line == ":c" || line == ":clear" || line == ":cls") {
#ifdef _WIN32
                std::system("cls");
#else
                std::system("clear");
#endif
                continue;
            }
            if (line == ":h" || line == ":help" || line == ":?") {
                help();
                continue;
            }
            if (line == ":env" || line == ":e") {
                if (shared_global_env) {
                    std::cout << "info: environment bindings:" << std::endl;
                    (*shared_global_env)->dump(std::cout, force_on_env_dump);
                } else {
                    std::cout << "info: environment is empty" << std::endl;
                }
                continue;
            }
            if (line == ":reload" || line == ":r") {
                shared_global_env.reset();
                std::cout << "info: environment reloaded" << std::endl;
                continue;
            }
            if (line == ":debug") {
                debug_enabled = !debug_enabled;
                continue;
            }
            if (line == ":force") {
                force_on_env_dump = !force_on_env_dump;
                continue;
            }
            if (line.empty()) {
                continue;
            }
            try {
                // TODO: In case of error do not EXIT from the program
                // Lex
                fe::lexer::Lexer lexer(line, fe::lexer::FromRepl{});
                const std::vector<fe::token::Token> tokens = lexer.lex_all();
                if (debug) {
                    for (const auto &tok: tokens) {
                        std::cout << tok << "\n";
                    }
                }
                // Parse
                fe::parser::Parser parser(tokens);
                if (debug) {
                    std::cout << parser.program << "\n";
                }
                // Interpret
                const auto [global_env, value] = intp::interp::interpret(
                    parser.program, shared_global_env, {.own_expr = true});
                std::cout << "===> " << value << "\n";
                shared_global_env = global_env;
            } catch (const std::exception &ex) {
                std::cerr << "error: " << ex.what() << "\n";
            }
        }
    }
}
