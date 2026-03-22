#pragma once

#include <optional>
#include <lbd/Context.hpp>
#include <lbd/frontend/token/Token.hpp>
#include <lbd/source/Buffer.hpp>

namespace lbd::frontend::lexer
{
    class Lexer
    {
    public:
        // TODO: Fix this options handling.
        explicit Lexer(source::Buffer& buffer, Context& context);

        [[nodiscard]] token::Token peek();

        [[nodiscard]] token::Token next();

        void advance();

        [[nodiscard]] bool hasNext() const;

    private:
        [[nodiscard]] token::Token lex();

        [[nodiscard]] char getCurrentCharacter() const;

        [[nodiscard]] char peekNextCurrentCharacter() const;

        [[nodiscard]] bool isEof() const;

        void advanceCursor();

        [[nodiscard]] token::TokenKind symbolToTokenKind(char symbol) const;

        source::Buffer& buffer;
        Context& context;
        source::Offset cursor = 0;
        source::RowNumber row = 1;
        source::ColumnNumber column = 1;
        std::optional<token::Token> currentToken; // Peeked token.
    };
}
