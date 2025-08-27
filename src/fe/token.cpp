#include <lbd/error.h>
#include <lbd/fe/token.h>
#include <utility>

namespace fe::token {
    Token::Token(TokenType typ, loc::Loc loc) : loc(std::move(loc)), typ(std::move(typ)) {
    }

    std::ostream &operator<<(std::ostream &os, const Token &token) {
        return os << token.loc << ":\t" << token.to_string();
    }

    std::string Token::to_string() const {
        return std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, Iden>) {
                return token::to_string<T>() + " <" + arg.value + ">";
            } else if constexpr (std::is_same_v<T, String>) {
                return token::to_string<T>() + " <\"" + arg.value + "\">";
            } else if constexpr (std::is_same_v<T, Float>) {
                return token::to_string<T>() + " <" + std::to_string(arg.value) + ">";
            } else {
                return token::to_string<T>();
            }
        }, typ);
    }

    template<typename T>
    std::string to_string() {
        if constexpr (std::is_same_v<T, std::monostate>) {
            return {"NULL"};
        } else if constexpr (std::is_same_v<T, Iden>) {
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
        } else if constexpr (std::is_same_v<T, OpenParen>) {
            return {"OPEN_PAREN"};
        } else if constexpr (std::is_same_v<T, CloseParen>) {
            return {"CLOSE_PAREN"};
        } else if constexpr (std::is_same_v<T, Eof>) {
            return {"EOF"};
        } else {
            STATIC_ASSERT_UNREACHABLE_T(T, "unhandled token");
            return {"UNKNOWN_TOKEN"}; // Unreachable
        }
    }
}
