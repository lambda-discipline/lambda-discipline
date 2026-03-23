#include <iostream>
#include <lbd/Driver.hpp>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/frontend/parser/Parser.hpp>
#include <lbd/frontend/program.h>
#include <lbd/runtime/interpreter.h>

namespace lbd
{
    Driver::Driver(Context& context) noexcept : context(context) {}

    runtime::Result Driver::run(const source::BufferId bufferId) const noexcept
    {
        const source::Buffer& buffer = context.getBufferManager().getBuffer(bufferId);
        frontend::lexer::Lexer lexer(context, buffer);

        // Render tokens if debug is enabled.
        // TODO: This debug printing, can be merged with parsing.
        if (context.getOptions().debug)
        {
            frontend::lexer::Lexer debugLexer(context, buffer);
            while (debugLexer.hasNext()) { std::cout << debugLexer.next() << std::endl; }
        }

        auto astNodes = frontend::parser::Parser(context, lexer).parse();
        const frontend::Program program(std::move(astNodes));
        // TODO: Render AST nodes if debug is enabled.

        return runtime::interpret(program, context);
    }
}
