#pragma once

#include <lbd/fe/loc.h>
#include <string>
#include <variant>

namespace fe::token {
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
        loc::Loc location;
        TokenType tokenType;

        Token(TokenType tokenType, loc::Loc location);

        friend std::ostream &operator<<(std::ostream &stream, const Token &token);

        [[nodiscard]] std::string toString() const;
    };

    template<typename T>
    std::string toString();
}
