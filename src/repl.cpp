#include <filesystem>
#include <fstream>
#include <iostream>
#include <lbd/repl.h>
#include <lbd/utils/term.h>
#include <lbd/fe/lexer.h>
#include <lbd/fe/parser.h>
#include <lbd/intp/interpreter.h>
#include <lbd/exceptions.h>

// TODO: Add support for clicking enter during lambda expression definition

#define on_off(val) ((val) ? "on " : "off")

namespace repl {
    static options::Options options_v;

    static void process_load_command(const std::string &arg,
                                     std::optional<std::shared_ptr<intp::interp::Env> > &shared_env) {
        const std::string &filepath = arg;
        if (!std::filesystem::exists(filepath)) {
            options_v.logger.error({}, "IO error: filepath ", filepath, "does not exists");
        }
        fe::lexer::Lexer lexer(filepath, fe::lexer::FromFile{});
        const auto tokens = lexer.lex_all();
        if (options_v.debug) {
            for (const auto &tok: tokens) {
                options_v.logger.debug(tok);
            }
        }
        fe::parser::Parser parser(tokens);
        const std::optional<std::shared_ptr<intp::interp::Env> > temp_env = shared_env;
        // Merge loaded_env into shared_env
        if (const auto [loaded_env, _, _result_options] = intp::interp::interpret(
                parser.program, temp_env, {.own_expr = true});
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
        options_v.logger.info("info: file loaded ", filepath);
    }


    void loop(const bool debug) {
        enable_virtual_terminal();

        static logs::Logger logger(false, true, false);
        options_v = {.own_expr = true, .force_on_env_dump = false, .debug = debug, .logger = logger};

        std::string line;
        std::optional<std::shared_ptr<intp::interp::Env> > shared_global_env = std::nullopt;

        options_v.logger.info("Welcome to lambda-discipline REPL.\nType :quit to exit.");

        while (true) {
            try {
                std::cout << colors::CYAN << "\n>> " << colors::RESET;
                if (!std::getline(std::cin, line)) {
                    break; // EOF (C-d / C-z)
                }
                if (line == ":q" || line == ":quit" || line == ":exit") {
                    options_v.logger.info("\nexiting REPL.");
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
                                    {":d, :debug", "", "Toggle debug mode"}
                                }, colors::GREEN);
                    std::cout << std::endl;
                    print_table({"Inspection Commands", "Argument", "Description"}, {
                                    {":e, :env", "", "Dump environment bindings"},
                                    {":force", "", "Force thunk evaluation on dump"}
                                }, colors::GREEN);
                    std::cout << std::endl;
                    print_table({"Options", "State", "Help"}, {
                                    {"debug", on_off(options_v.debug), "use :debug to toggle"},
                                    {"force-on-env-dump", on_off(options_v.force_on_env_dump), "use :force to toggle"}
                                }, colors::GREEN);
                    continue;
                }
                if (line == ":env" || line == ":e") {
                    std::cout << std::endl;
                    if (shared_global_env) {
                        print_table({"Symbol", "Thunk"}, (*shared_global_env)->to_vector(options_v.force_on_env_dump),
                                    colors::GREEN);
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
                    options_v.debug = !options_v.debug;
                    continue;
                }
                if (line == ":force") {
                    options_v.force_on_env_dump = !options_v.force_on_env_dump;
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
                // Lex
                fe::lexer::Lexer lexer_v(line, fe::lexer::FromRepl{}, options_v);
                const std::vector<fe::token::Token> tokens = lexer_v.lex_all();
                if (options_v.debug) {
                    for (const auto &tok: tokens) {
                        options_v.logger.debug(tok);
                    }
                }

                // Parse
                fe::parser::Parser parser_v(tokens, options_v);
                if (options_v.debug) {
                    options_v.logger.debug(parser_v.program);
                }

                // Interpret
                const auto [global_env, value, result_options] = intp::interp::interpret(
                    parser_v.program, shared_global_env, options_v);
                if (result_options.side_effects) {
                    std::cout << std::endl;
                }
                std::cout << colors::GREEN << "=> " << value << colors::RESET << std::endl;
                shared_global_env = global_env;
            } catch (const ControlledExit &) {
            } catch (const std::exception &ex) {
                options_v.logger.error({}, "error: ", ex.what());;
            }
        }
    }
}
