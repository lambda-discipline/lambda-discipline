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
    [[nodiscard]] explicit Lexer(Context &context, const source::Buffer &buffer) noexcept;

    [[nodiscard]] token::Token peek() noexcept;

    [[nodiscard]] token::Token next() noexcept;

    void advance() noexcept;

    [[nodiscard]] bool hasNext() const noexcept;

    [[nodiscard]] const source::Buffer &getBuffer() const noexcept;

  private:
    [[nodiscard]] token::Token lex() noexcept;

    [[nodiscard]] char getCurrentCharacter() const noexcept;

    [[nodiscard]] char peekNextCurrentCharacter() const noexcept;

    [[nodiscard]] bool isEof() const noexcept;

    void advanceCursor() noexcept;

    [[nodiscard]] token::TokenKind symbolToTokenKind(char symbol) const noexcept;

    Context &context;
    const source::Buffer &buffer;
    source::Offset cursor = 0;
    source::RowNumber row = 1;
    source::ColumnNumber column = 1;
    std::optional<token::Token> currentToken; // Peeked token.
  };
}