#include <cctype>
#include <fstream>
#include <utility>
#include <lbd/logs.h>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/source/Range.hpp>

#include "lbd/utils/string_escape.h"

namespace lbd::frontend::lexer
{
    static bool isIdentifierStart(const char c) { return std::isalpha(c) || c == '_'; }

    static bool isIdentifier(const char c) { return std::isalnum(c) || c == '_'; }

    static bool isNumberStart(const char c) { return std::isdigit(c) || c == '-'; }

    Lexer::Lexer(source::Buffer& buffer, Context& context) : buffer(buffer), context(context) {}

    token::Token Lexer::peek()
    {
        if (!currentToken.has_value()) currentToken = lex();
        return currentToken.value();
    }

    token::Token Lexer::next()
    {
        if (currentToken.has_value())
        {
            const token::Token token = currentToken.value();
            advance();
            return token;
        }
        return lex();
    }

    void Lexer::advance() { currentToken = std::nullopt; }

    bool Lexer::hasNext() const { return !isEof(); }

    token::Token Lexer::lex()
    {
        while (std::isspace(getCurrentCharacter())) advanceCursor();

        const source::Location beginLocation(buffer.getFileId(), cursor, row, column);

        // TODO: Add context guards.

        if (isEof()) return {token::TokenKind::END_OF_FILE, "", source::Range(beginLocation, beginLocation)};

        // Identifier.
        if (isIdentifierStart(getCurrentCharacter()))
        {
            const source::Offset beginOffset = cursor;
            while (!isEof() && isIdentifier(getCurrentCharacter())) advanceCursor();
            const auto lexeme = buffer.getText().substr(beginOffset, cursor - beginOffset);
            const source::Location endLocation(buffer.getFileId(), cursor, row, column);
            return {token::TokenKind::IDENTIFIER, lexeme, source::Range(beginLocation, endLocation)};
        }

        // Number.
        if (isNumberStart(getCurrentCharacter()))
        {
            // Comment or Arrow.
            if (getCurrentCharacter() == '-' && (peekNextCurrentCharacter() == '-'
                || peekNextCurrentCharacter() == '>'))
                goto LexSymbol;
            if (getCurrentCharacter() == '-' && !std::isdigit(peekNextCurrentCharacter()))
            {
                context.getOptions().logger.error(beginLocation, "syntax error: expected digit after - (dash).");
            }
            const source::Offset beginOffset = cursor;
            advanceCursor(); // Advance to consume possible '-'.
            while (!isEof() && std::isdigit(getCurrentCharacter())) advanceCursor();
            if (getCurrentCharacter() == '.')
            {
                advanceCursor(); // Consume '.'
                while (!isEof() && std::isdigit(getCurrentCharacter())) advanceCursor();
            }
            const auto lexeme = buffer.getText().substr(beginOffset, cursor - beginOffset);
            const source::Location endLocation(buffer.getFileId(), cursor, row, column);
            return {token::TokenKind::NUMBER, lexeme, source::Range(beginLocation, endLocation)};
        }

        // String literal.
        if (getCurrentCharacter() == '"')
        {
            const source::Offset beginOffset = cursor;
            advanceCursor(); // Consume '"'
            while (!isEof() && getCurrentCharacter() != '"') advanceCursor();
            if (getCurrentCharacter() != '"')
            {
                context.getOptions().logger.error(beginLocation, "syntax error: unbalanced quote");
            }
            advanceCursor(); // Consume '"'
            // +1 and -1 for excluding the quotes.
            const auto lexeme = buffer.getText().substr(beginOffset + 1, cursor - beginOffset - 2);
            const source::Location endLocation(buffer.getFileId(), cursor, row, column);
            return {token::TokenKind::STRING, lexeme, source::Range(beginLocation, endLocation)};
        }

    LexSymbol:
        switch (const char symbol = getCurrentCharacter())
        {
            case ':':
            case '=':
            case '\\':
            case '.':
            case '(':
            case ')':
                {
                    advanceCursor();
                    const auto lexeme = std::string(1, symbol);
                    const source::Location endLocation(buffer.getFileId(), cursor, row, column);
                    return {symbolToTokenKind(symbol), lexeme, source::Range(beginLocation, endLocation)};
                }
            case '-':
                {
                    advanceCursor(); // Consume '-'
                    switch (getCurrentCharacter())
                    {
                        case '-':
                            // Comment.
                            while (!isEof() && getCurrentCharacter() != '\n') advanceCursor();
                            return lex();
                        case '>':
                            {
                                // Arrow.
                                advanceCursor(); // Consume '>'.
                                const source::Location endLocation(buffer.getFileId(), cursor, row, column);
                                return {token::TokenKind::ARROW, "->", source::Range(beginLocation, endLocation)};
                            }
                        default:
                            break;
                    }
                }
            default:
                break;
        }
        context.getOptions().logger.error(beginLocation, "syntax error: unexpected character ", getCurrentCharacter());
    }

    char Lexer::getCurrentCharacter() const { return isEof() ? '\0' : buffer[cursor]; }

    char Lexer::peekNextCurrentCharacter() const { return cursor + 1 < buffer.getSize() ? buffer[cursor + 1] : '\0'; }

    bool Lexer::isEof() const { return cursor >= buffer.getSize(); }

    void Lexer::advanceCursor()
    {
        if (getCurrentCharacter() == '\n')
        {
            row++;
            column = 1;
        }
        else
        {
            column++;
        }
        cursor++;
    }

    token::TokenKind Lexer::symbolToTokenKind(const char symbol) const
    {
        switch (symbol)
        {
            case ':': return token::TokenKind::COLON;
            case '=': return token::TokenKind::ASSIGNMENT;
            case '\\': return token::TokenKind::BACKWARD_SLASH;
            case '.': return token::TokenKind::DOT;
            case '(': return token::TokenKind::OPEN_PARENTHESIS;
            case ')': return token::TokenKind::CLOSE_PARENTHESIS;
            default:
                const source::Location beginLocation(buffer.getFileId(), cursor, row, column);
                context.getOptions().logger.error(beginLocation, "syntax error: unexpected character ", symbol);
        }
    }
}
