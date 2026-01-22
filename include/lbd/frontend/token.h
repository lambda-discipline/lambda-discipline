#pragma once

#include <lbd/frontend/location.h>
#include <string>
#include <variant>

namespace frontend::token {
    struct Identifier {
        std::string value;
    };

    struct String {
        std::string value;
    };

    struct Colon {
    };

    struct Equal {
    };

    struct Float {
        double value;
    };

    struct Arrow {
    };

    struct BackwardSlash {
    };

    struct Dot {
    };

    struct OpenParenthesis {
    };

    struct CloseParenthesis {
    };

    struct Eof {
    };

    using TokenType = std::variant<
        std::monostate, // analogous to NULL
        Identifier,
        String,
        Colon,
        Equal,
        Float,
        Arrow,
        BackwardSlash,
        Dot,
        OpenParenthesis,
        CloseParenthesis,
        Eof
    >;

    struct Token {
        Location location;
        TokenType tokenType;

        Token(TokenType tokenType, Location location);

        friend std::ostream &operator<<(std::ostream &stream, const Token &token);

        [[nodiscard]] std::string toString() const;
    };

    template<typename T>
    std::string toString();
}
