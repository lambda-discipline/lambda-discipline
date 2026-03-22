#pragma once

#include <string_view>
#include <utility>

namespace lbd::frontend::token
{
    enum class TokenKind
    {
        IDENTIFIER,
        STRING,
        NUMBER,
        COLON,
        ASSIGNMENT, // =
        ARROW,
        BACKWARD_SLASH,
        DOT,
        OPEN_PARENTHESIS,
        CLOSE_PARENTHESIS,
        END_OF_FILE,
        UNKNOWN, // Used as a default TokenKind inside Lexer.
    };

    constexpr std::string_view tokenKindToString(const TokenKind tokenKind)
    {
        switch (tokenKind)
        {
            case TokenKind::IDENTIFIER: return "IDENTIFIER";
            case TokenKind::STRING: return "STRING";
            case TokenKind::NUMBER: return "NUMBER";
            case TokenKind::COLON: return "COLON";
            case TokenKind::ASSIGNMENT: return "ASSIGNMENT";
            case TokenKind::ARROW: return "ARROW";
            case TokenKind::BACKWARD_SLASH: return "BACKWARD_SLASH";
            case TokenKind::DOT: return "DOT";
            case TokenKind::OPEN_PARENTHESIS: return "OPEN_PARENTHESIS";
            case TokenKind::CLOSE_PARENTHESIS: return "CLOSE_PARENTHESIS";
            case TokenKind::END_OF_FILE: return "END_OF_FILE";
            case TokenKind::UNKNOWN: return "UNKNOWN";
        }
        std::unreachable();
    }
}
