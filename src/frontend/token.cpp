#include <lbd/error.h>
#include <lbd/frontend/token.h>
#include <iostream>
#include <utility>

namespace frontend::token {
    Token::Token(TokenType tokenType, Location location) : location(std::move(location)),
                                                           tokenType(std::move(tokenType)) {
    }

    std::ostream &operator<<(std::ostream &stream, const Token &token) {
        return stream << token.location << ":\t" << token.toString();
    }

    std::string Token::toString() const {
        return std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, Identifier>) {
                return token::toString<T>() + " <" + arg.value + ">";
            } else if constexpr (std::is_same_v<T, String>) {
                return token::toString<T>() + " <\"" + arg.value + "\">";
            } else if constexpr (std::is_same_v<T, Float>) {
                return token::toString<T>() + " <" + std::to_string(arg.value) + ">";
            } else {
                return token::toString<T>();
            }
        }, tokenType);
    }

    template<typename T>
    std::string toString() {
        if constexpr (std::is_same_v<T, std::monostate>) {
            return {"NULL"};
        } else if constexpr (std::is_same_v<T, Identifier>) {
            return {"ID"};
        } else if constexpr (std::is_same_v<T, String>) {
            return {"STRING"};
        } else if constexpr (std::is_same_v<T, Colon>) {
            return {"COLON"};
        } else if constexpr (std::is_same_v<T, Equal>) {
            return {"EQUAL"};
        } else if constexpr (std::is_same_v<T, Float>) {
            return {"FLOAT"};
        } else if constexpr (std::is_same_v<T, Arrow>) {
            return {"ARROW"};
        } else if constexpr (std::is_same_v<T, BackwardSlash>) {
            return {"BACKWARD_SLASH"};
        } else if constexpr (std::is_same_v<T, Dot>) {
            return {"DOT"};
        } else if constexpr (std::is_same_v<T, OpenParenthesis>) {
            return {"OPEN_PAREN"};
        } else if constexpr (std::is_same_v<T, CloseParenthesis>) {
            return {"CLOSE_PAREN"};
        } else if constexpr (std::is_same_v<T, Eof>) {
            return {"EOF"};
        } else {
            STATIC_ASSERT_UNREACHABLE_T(T, "unhandled token");
            return {"UNKNOWN_TOKEN"}; // Unreachable
        }
    }
}
