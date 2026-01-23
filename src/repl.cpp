#include <filesystem>
#include <fstream>
#include <iostream>
#include <lbd/repl.h>
#include <lbd/utils/term.h>
#include <lbd/frontend/lexer.h>
#include <lbd/frontend/parser.h>
#include <lbd/interpreter/interpreter.h>
#include <lbd/exceptions.h>

#define onOrOff(val) ((val) ? "on " : "off")

// TODO: Option :t for display type information of a symbol.

namespace repl {
    static global::Options optionsValue;

    static void processLoadCommand(const std::string &argument,
                                   std::optional<std::shared_ptr<interpreter::Environment> > &sharedEnvironment) {
        const std::string &filepath = argument;
        global::Options subOptions = optionsValue;
        subOptions.logger.showLocation = true;

        if (!std::filesystem::exists(filepath)) {
            subOptions.logger.error({}, "IO error: filepath ", filepath, " does not exist");
        }

        frontend::Lexer lexer = frontend::Lexer::fromFile(filepath, subOptions);
        const auto tokens = lexer.lex();
        if (subOptions.debug) {
            for (const auto &token: tokens) {
                subOptions.logger.debug(token);
            }
        }

        frontend::Parser parser(tokens, subOptions);
        if (subOptions.debug) {
            for (const auto &node: parser.program.nodes) {
                subOptions.logger.debug(node);
            }
        }

        const std::optional<std::shared_ptr<interpreter::Environment> > temporaryEnvironment = sharedEnvironment;
        // Merge loaded_env into shared_env
        if (const auto [loadedEnvironment, _, resultantOptions] = interpreter::interpret(
            parser.program, temporaryEnvironment, subOptions); loadedEnvironment) {
            if (!sharedEnvironment) {
                sharedEnvironment = loadedEnvironment;
            } else {
                // Merge all Bindings from loaded_env into shared_env
                for (const auto &[first, second]: loadedEnvironment->table) {
                    (*sharedEnvironment)->bind(first, second);
                }
            }
            if (resultantOptions.sideEffects) {
                std::cout << std::endl;
            }
        }

        subOptions.logger.info("info: file loaded ", filepath);
    }

    static int computeParenthesisDepth(const std::string &data) {
        int depth = 0;
        for (const char c: data) {
            if (c == '(') depth++;
            else if (c == ')') depth = std::max(0, depth - 1);
        }
        return depth;
    }

    static std::string trim(const std::string &data) {
        size_t start = 0;
        while (start < data.size() && std::isspace(static_cast<unsigned char>(data[start]))) start++;
        size_t end = data.size();
        while (end > start && std::isspace(static_cast<unsigned char>(data[end - 1]))) end--;
        return data.substr(start, end - start);
    }

    void loop(const bool debug) {
        enableVirtualTerminal();

        static logs::Logger logger(false, true, false);
        optionsValue = {.ownExpression = true, .forceOnEnvironmentDump = false, .debug = debug, .logger = logger};

        std::string line, buffer;
        size_t indentLevel = 0;
        std::optional<std::shared_ptr<interpreter::Environment> > sharedGlobalEnvironment = std::nullopt;

        optionsValue.logger.info("Welcome to lambda-discipline REPL.\nType :quit to exit.");

        while (true) {
            try {
                // Prompt depends on whether we are continuing a buffer
                if (buffer.empty()) {
                    std::cout << colors::CYAN << "\n>> " << colors::RESET;
                } else {
                    std::cout << colors::CYAN << ".. " << colors::RESET;
                }
                for (size_t i = 0; i < indentLevel; i++) std::cout << "  ";

                if (!std::getline(std::cin, line)) break; // EOF (C-d / C-z)

                // REPL arguments parsing
                {
                    if (line == ":q" || line == ":quit" || line == ":exit") {
                        optionsValue.logger.info("\nexiting REPL.");
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
                        printTable({"General Commands", "Argument", "Description"}, {
                                       {":q, :quit, :exit", "", "Exit the REPL"},
                                       {":c, :clear, :cls", "", "Clear the screen"},
                                       {":h, :help, :?", "", "Display this help message"},
                                       {":l, :load", "<filepath>", "Load file into REPL"},
                                       {":r, :reset", "", "Reset environment"},
                                       {":d, :debug", "", "Toggle debug mode"}
                                   }, colors::GREEN);
                        std::cout << std::endl;
                        printTable({"Inspection Commands", "Argument", "Description"}, {
                                       {":e, :env", "", "Dump environment bindings"},
                                       {":force", "", "Force thunk evaluation on dump"}
                                   }, colors::GREEN);
                        std::cout << std::endl;
                        printTable({"Options", "State", "Help"}, {
                                       {"debug", onOrOff(optionsValue.debug), "use :debug to toggle"},
                                       {
                                           "force-on-env-dump", onOrOff(optionsValue.forceOnEnvironmentDump),
                                           "use :force to toggle"
                                       }
                                   }, colors::GREEN);
                        continue;
                    }

                    if (line == ":env" || line == ":e") {
                        std::cout << std::endl;
                        if (sharedGlobalEnvironment) {
                            printTable({"Symbol", "Thunk"},
                                       (*sharedGlobalEnvironment)->toVector(optionsValue.forceOnEnvironmentDump),
                                       colors::GREEN);
                        } else {
                            printTable({"Symbol", "Thunk"}, {{"Empty"}}, colors::GREEN);
                        }
                        continue;
                    }

                    if (line == ":reset" || line == ":r") {
                        sharedGlobalEnvironment.reset();
                        continue;
                    }

                    if (line == ":debug" || line == ":d") {
                        optionsValue.debug = !optionsValue.debug;
                        continue;
                    }

                    if (line == ":force") {
                        optionsValue.forceOnEnvironmentDump = !optionsValue.forceOnEnvironmentDump;
                        continue;
                    }

                    if (line.rfind(":load ", 0) == 0) {
                        processLoadCommand(line.substr(6), sharedGlobalEnvironment);
                        continue;
                    }

                    if (line.rfind(":l ", 0) == 0) {
                        processLoadCommand(line.substr(3), sharedGlobalEnvironment);
                        continue;
                    }
                }

                if (line.empty()) continue;

                std::string trimmed = trim(line);
                // Continue the line with trailing '`'.
                bool continuation = !trimmed.empty() && trimmed.back() == '`';
                if (continuation) {
                    trimmed.pop_back();
                }
                buffer += trimmed + "\n";

                if (continuation) {
                    // Indent level depends on whether the current line ends with '.' after trimming and removing trailing '`'.
                    // Additionally, it also depends upon the current parenthesis depth.
                    indentLevel = computeParenthesisDepth(buffer);
                    trimmed = trim(trimmed);
                    if (!trimmed.empty() && trimmed.back() == '.') indentLevel++;
                    continue;
                }
                indentLevel = 0;
                line = buffer;
                buffer.clear();

                // Lex
                frontend::Lexer lexerValue = frontend::Lexer::fromRepl(line, optionsValue);
                const std::vector<frontend::token::Token> tokens = lexerValue.lex();
                if (optionsValue.debug) {
                    for (const auto &tok: tokens) {
                        optionsValue.logger.debug(tok);
                    }
                }

                // Parse
                frontend::Parser parserValue(tokens, optionsValue);
                if (optionsValue.debug) {
                    optionsValue.logger.debug(parserValue.program);
                }

                // Interpret
                const auto [globalEnvironment, value, resultantOptions] = interpreter::interpret(
                    parserValue.program, sharedGlobalEnvironment, optionsValue);
                if (resultantOptions.sideEffects) {
                    std::cout << std::endl;
                }
                std::cout << colors::GREEN << "=> " << value << colors::RESET << std::endl;
                sharedGlobalEnvironment = globalEnvironment;
            } catch (const ControlledExit &) {
            } catch (const std::exception &ex) {
                optionsValue.logger.error({}, "error: ", ex.what());
            }
        }
    }
}
