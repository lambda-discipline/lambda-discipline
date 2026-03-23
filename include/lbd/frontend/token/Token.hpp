#pragma once

#include <lbd/frontend/token/TokenKind.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::frontend::token
{
    struct Token
    {
        TokenKind kind;
        std::string lexeme;
        source::Range range;

        // For debugging.
        friend std::ostream& operator<<(std::ostream& outputStream, const Token& token);
    };
}
