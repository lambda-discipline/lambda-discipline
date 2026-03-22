#include <iostream>
#include <string>
#include <vector>
#include <lbd/cmd.h>
#include <lbd/docs.h>
#include <lbd/repl.h>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/frontend/parser/Parser.hpp>
#include <lbd/runtime/interpreter.h>

const std::string& programName = "lbd";

int main(const int argc, char** argv)
{
    const auto& [filepath, show_help, repl, debug, generateDocs] = cmd::parseArguments(argc, argv, programName);
    if (show_help)
    {
        cmd::printHelp(std::cout, argv[0]);
        return EXIT_SUCCESS;
    }
    if (generateDocs)
    {
        lbd::docs::dumpDocs(std::cout);
        return EXIT_SUCCESS;
    }

    lbd::Context context{lbd::Options()};

    if (repl)
    {
        lbd::repl::loop(context, debug);
    }
    else
    {
        const lbd::source::FileId fileId = context.getSourceManager().loadFile(*filepath);
        lbd::source::Buffer buffer(fileId, context.getSourceManager());
        lbd::frontend::lexer::Lexer lexer(buffer, context);

        // lbd::frontend::lexer::Lexer debugLexer(buffer, context);
        // while (debugLexer.hasNext())
        // {
        //     std::cout << debugLexer.next() << std::endl;
        // }

        // const std::vector<lbd::frontend::token::Token> tokens = lexerValue.lex();
        // if (debug)
        // {
        //     for (const frontend::token::Token& token : tokens)
        //     {
        //         std::cout << token << std::endl;
        //     }
        // }

        auto astNodes = lbd::frontend::parser::Parser(lexer, context).parse();
        lbd::frontend::Program program(std::move(astNodes));

        // if (debug)
        // {
        //     std::cout << program << std::endl;
        // }
        // Interpret

        auto result = lbd::runtime::interpret(program);
        return EXIT_SUCCESS;
    }
}
