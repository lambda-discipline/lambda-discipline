#include <lbd/frontend/token/Token.hpp>

namespace lbd::frontend::token
{
    Token::Token(const TokenKind kind, const std::string_view lexeme,
                 const source::Range& range) : kind(kind), lexeme(lexeme), range(range) {}

    TokenKind Token::getKind() const { return kind; }

    std::string_view Token::getLexeme() const { return lexeme; }

    source::Range Token::getRange() const { return range; }

    std::ostream& operator<<(std::ostream& outputStream, const Token& token)
    {
        return outputStream << token.getRange() << " <" << tokenKindToString(token.getKind()) << "> "
            << token.getLexeme() << std::endl;
    }
}
