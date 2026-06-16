#include <lbd/frontend/token/Token.hpp>

namespace lbd::frontend::token
{
  std::ostream &operator<<(std::ostream &outputStream, const Token &token)
  {
    return outputStream << token.range << " <" << tokenKindToString(token.kind) << "> " << token.lexeme;
  }
}