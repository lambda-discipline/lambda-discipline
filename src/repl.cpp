#include <filesystem>
#include <fstream>
#include <iostream>
#include <lbd/repl.h>
#include <lbd/term.h>
#include <lbd/fe/lexer.h>
#include <lbd/fe/parser.h>
#include <lbd/intp/interpreter.h>

// TODO: Add support for clicking enter during lambda expression definition

namespace repl {
    static void process_load_command(const std::string &arg,
                                     std::optional<std::shared_ptr<intp::interp::Env> > &shared_env) {
        try {
            const std::string &filepath = arg;
            if (!std::filesystem::exists(filepath)) {
                std::cerr << colors::RED << "error: filepath " << filepath << " does not exist"
                        << colors::RESET << std::endl;
                return;
            }
            fe::lexer::Lexer lexer(filepath, fe::lexer::FromFile{});
            const auto tokens = lexer.lex_all();
            // TODO: Accept Options and Add a debug flag for printing Lexed Tokens
            fe::parser::Parser parser(tokens);
            const std::optional<std::shared_ptr<intp::interp::Env> > temp_env = shared_env;
            // Merge loaded_env into shared_env
            if (const auto [loaded_env, _] = intp::interp::interpret(parser.program, temp_env, {.own_expr = true});
                loaded_env) {
                if (!shared_env) {
                    shared_env = loaded_env;
                } else {
                    // Merge all Bindings from loaded_env into shared_env
                    for (const auto &[fst, snd]: loaded_env->table) {
                        (*shared_env)->bind(fst, snd);
                    }
                }
            }
            std::cout << colors::BLUE << "info: file loaded: " << filepath << colors::RESET << std::endl;
        } catch (const std::exception &ex) {
            std::cerr << colors::RED << "error: could not load file: " << ex.what() << colors::RESET << std::endl;
        }
    }

#define on_off(val) ((val) ? "on " : "off")

    void loop(const bool debug) {
        enable_virtual_terminal();
        static bool force_on_env_dump = false;
        static bool debug_enabled = false;
        std::cout << colors::BLUE << "Welcome to lambda-discipline REPL.\nType :quit to exit."
                << colors::RESET << std::endl;
        std::string line;
        std::optional<std::shared_ptr<intp::interp::Env> > shared_global_env = std::nullopt;
        while (true) {
            std::cout << colors::CYAN << "\n>> " << colors::RESET;
            if (!std::getline(std::cin, line)) {
                break; // EOF (C-d / C-z)
            }
            if (line == ":q" || line == ":quit" || line == ":exit") {
                std::cout << colors::BLUE << "exiting REPL" << colors::RESET << std::endl;
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
                std::cout << std::endl;
                print_table({"General Commands", "Argument", "Description"}, {
                                {":q, :quit, :exit", "", "Clear the screen"},
                                {":c, :clear, :cls", "", "Exit the REPL"},
                                {":h, :help, :?", "", "Display this help message"},
                                {":l, :load", "<filepath>", "Load file into REPL"},
                                {":r, :reset", "", "Reset environment"},
                                {":d, :debug", "", "Toogle debug mode"}
                            }, colors::GREEN);
                std::cout << std::endl;
                print_table({"Inspection Commands", "Argument", "Description"}, {
                                {":e, :env", "", "Dump environment bindings"},
                                {":force", "", "Force thunk evaluation on dump"}
                            }, colors::GREEN);
                std::cout << std::endl;
                print_table({"Options", "State", "Help"}, {
                                {"debug", on_off(debug_enabled), "use :debug to toggle"},
                                {"force-on-env-dump", on_off(force_on_env_dump), "use :force to toggle"}
                            }, colors::GREEN);
                continue;
            }
            if (line == ":env" || line == ":e") {
                std::cout << std::endl;
                if (shared_global_env) {
                    print_table({"Symbol", "Thunk"}, (*shared_global_env)->to_vector(force_on_env_dump), colors::GREEN);
                } else {
                    print_table({"Symbol", "Thunk"}, {{"Empty"}}, colors::GREEN);
                }
                continue;
            }
            if (line == ":reset" || line == ":r") {
                shared_global_env.reset();
                continue;
            }
            if (line == ":debug" || line == ":d") {
                debug_enabled = !debug_enabled;
                continue;
            }
            if (line == ":force") {
                force_on_env_dump = !force_on_env_dump;
                continue;
            }
            if (line.rfind(":load ", 0) == 0) {
                process_load_command(line.substr(6), shared_global_env);
                continue;
            }
            if (line.rfind(":l ", 0) == 0) {
                process_load_command(line.substr(3), shared_global_env);
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
                        std::cout << colors::YELLOW << tok << colors::RESET << std::endl;
                    }
                }
                // Parse
                fe::parser::Parser parser(tokens);
                if (debug) {
                    std::cout << colors::YELLOW << parser.program << colors::RESET << std::endl;
                }
                // Interpret
                // TODO: Add flag to disable printing in REPL (disabled by default)
                const auto [global_env, value] = intp::interp::interpret(
                    parser.program, shared_global_env, {.own_expr = true});
                std::cout << colors::GREEN << "=> " << value << colors::RESET << std::endl;
                shared_global_env = global_env;
            } catch (const std::exception &ex) {
                std::cerr << colors::RED << "error: " << ex.what() << colors::RESET << std::endl;
            }
        }
    }
}
