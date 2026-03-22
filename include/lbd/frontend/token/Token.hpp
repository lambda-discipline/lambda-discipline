#pragma once

#include <string_view>
#include <lbd/frontend/token/TokenKind.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::frontend::token
{
    class Token
    {
    public:
        Token(TokenKind kind, std::string_view lexeme, const source::Range& range);

        [[nodiscard]] TokenKind getKind() const;

        [[nodiscard]] std::string_view getLexeme() const;

        [[nodiscard]] source::Range getRange() const;

        // For debugging.
        friend std::ostream& operator<<(std::ostream& outputStream, const Token& token);

    private:
        TokenKind kind;
        std::string_view lexeme;
        source::Range range;
    };
}
