#pragma once

#include <lbd/fe/loc.h>
#include <string>
#include <variant>

namespace fe::token {
    struct Iden {
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

    struct OpenParen {
    };

    struct CloseParen {
    };

    struct Eof {
    };

    using TokenType = std::variant<
        std::monostate, // analogous to NULL
        Iden,
        String,
        Colon,
        Equal,
        Float,
        Arrow,
        BackwardSlash,
        Dot,
        OpenParen,
        CloseParen,
        Eof
    >;

    struct Token {
        loc::Loc loc;
        TokenType typ;

        Token(TokenType typ, loc::Loc loc);

        friend std::ostream &operator<<(std::ostream &os, const Token &token);

        [[nodiscard]] std::string to_string() const;
    };

    template<typename T>
    std::string to_string();
}
