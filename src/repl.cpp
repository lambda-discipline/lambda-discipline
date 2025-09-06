#include <filesystem>
#include <fstream>
#include <iostream>
#include <lbd/repl.h>
#include <lbd/utils/term.h>
#include <lbd/fe/lexer.h>
#include <lbd/fe/parser.h>
#include <lbd/intp/interpreter.h>
#include <lbd/exceptions.h>

#define on_off(val) ((val) ? "on " : "off")

// TODO: Option :t for display type information of a symbol

namespace repl {
    static options::Options options_v;

    static void process_load_command(const std::string &arg,
                                     std::optional<std::shared_ptr<intp::interp::Env> > &shared_env) {
        const std::string &filepath = arg;
        options::Options sub_options = options_v;
        sub_options.logger.show_loc = true;

        if (!std::filesystem::exists(filepath)) {
            sub_options.logger.error({}, "IO error: filepath ", filepath, " does not exist");
        }

        fe::lexer::Lexer lexer(filepath, fe::lexer::FromFile{}, sub_options);
        const auto tokens = lexer.lex_all();
        if (sub_options.debug) {
            for (const auto &tok: tokens) {
                sub_options.logger.debug(tok);
            }
        }

        fe::parser::Parser parser(tokens, sub_options);
        if (sub_options.debug) {
            for (const auto &node: parser.program.nodes) {
                sub_options.logger.debug(node);
            }
        }

        const std::optional<std::shared_ptr<intp::interp::Env> > temp_env = shared_env;
        // Merge loaded_env into shared_env
        if (const auto [loaded_env, _, result_options] = intp::interp::interpret(
                parser.program, temp_env, sub_options);
            loaded_env) {
            if (!shared_env) {
                shared_env = loaded_env;
            } else {
                // Merge all Bindings from loaded_env into shared_env
                for (const auto &[fst, snd]: loaded_env->table) {
                    (*shared_env)->bind(fst, snd);
                }
            }
            if (result_options.side_effects) {
                std::cout << std::endl;
            }
        }

        sub_options.logger.info("info: file loaded ", filepath);
    }

    static int compute_paren_depth(const std::string &s) {
        int depth = 0;
        for (const char c: s) {
            if (c == '(') depth++;
            else if (c == ')') depth = std::max(0, depth - 1);
        }
        return depth;
    }

    static std::string trim(const std::string &s) {
        size_t start = 0;
        while (start < s.size() && std::isspace(static_cast<unsigned char>(s[start]))) start++;
        size_t end = s.size();
        while (end > start && std::isspace(static_cast<unsigned char>(s[end - 1]))) end--;
        return s.substr(start, end - start);
    }

    void loop(const bool debug) {
        enable_virtual_terminal();

        static logs::Logger logger(false, true, false);
        options_v = {.own_expr = true, .force_on_env_dump = false, .debug = debug, .logger = logger};

        std::string line, buffer;
        size_t indent_level = 0;
        std::optional<std::shared_ptr<intp::interp::Env> > shared_global_env = std::nullopt;

        options_v.logger.info("Welcome to lambda-discipline REPL.\nType :quit to exit.");

        while (true) {
            try {
                // Prompt depends on whether we are continuing a buffer
                if (buffer.empty()) {
                    std::cout << colors::CYAN << "\n>> " << colors::RESET;
                } else {
                    std::cout << colors::CYAN << ".. " << colors::RESET;
                }
                for (size_t i = 0; i < indent_level; i++) std::cout << "  ";

                if (!std::getline(std::cin, line)) break; // EOF (C-d / C-z)

                // REPL arguments parsing
                {
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
                                        {
                                            "force-on-env-dump", on_off(options_v.force_on_env_dump),
                                            "use :force to toggle"
                                        }
                                    }, colors::GREEN);
                        continue;
                    }

                    if (line == ":env" || line == ":e") {
                        std::cout << std::endl;
                        if (shared_global_env) {
                            print_table({"Symbol", "Thunk"},
                                        (*shared_global_env)->to_vector(options_v.force_on_env_dump),
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
                }

                if (line.empty()) continue;

                std::string trimmed = trim(line);
                // Continue the line with trailing '`'
                bool continuation = !trimmed.empty() && trimmed.back() == '`';
                if (continuation) {
                    trimmed.pop_back();
                }
                buffer += trimmed + "\n";

                if (continuation) {
                    // Indent level depends on whether the current line ends with '.' after trimming and removing trailing '`'.
                    // Additionally, it also depends upon the current parenthesis depth.
                    indent_level = compute_paren_depth(buffer);
                    trimmed = trim(trimmed);
                    if (!trimmed.empty() && trimmed.back() == '.') indent_level++;
                    continue;
                }
                indent_level = 0;
                line = buffer;
                buffer.clear();

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
                options_v.logger.error({}, "error: ", ex.what());
            }
        }
    }
}
