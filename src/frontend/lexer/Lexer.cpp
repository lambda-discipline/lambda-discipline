#include <cctype>
#include <fstream>
#include <utility>
#include <fmt/core.h>
#include <lbd/diagnostics/ContextGuard.hpp>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::frontend::lexer
{
    static constexpr bool isIdentifierStart(const char c) noexcept { return std::isalpha(c) || c == '_'; }
    static constexpr bool isIdentifier(const char c) noexcept { return std::isalnum(c) || c == '_'; }
    static constexpr bool isNumberStart(const char c) noexcept { return std::isdigit(c) || c == '-'; }

    Lexer::Lexer(Context& context, const source::Buffer& buffer) noexcept : context(context), buffer(buffer) {}

    token::Token Lexer::peek() noexcept
    {
        if (!currentToken.has_value()) currentToken = lex();
        return currentToken.value();
    }

    token::Token Lexer::next() noexcept
    {
        if (currentToken.has_value())
        {
            const token::Token token = currentToken.value();
            advance();
            return token;
        }
        return lex();
    }

    void Lexer::advance() noexcept { currentToken = std::nullopt; }

    bool Lexer::hasNext() const noexcept { return !isEof(); }

    const source::Buffer& Lexer::getBuffer() const noexcept { return buffer; }

    token::Token Lexer::lex() noexcept
    {
        while (std::isspace(getCurrentCharacter())) advanceCursor();

        const source::Location beginLocation{buffer.id, cursor};

        // Adding Context Guard for diagnostics.
        diagnostics::ContextGuard contextGuard(
            context.getDiagnosticEmitter(),
            source::Range(beginLocation, {buffer.id, static_cast<source::Offset>(buffer.getSize())}),
            fmt::format("While lexing `{}`.", buffer.name));

        if (isEof()) return {token::TokenKind::END_OF_FILE, "", {beginLocation, beginLocation}};

        // Identifier.
        if (isIdentifierStart(getCurrentCharacter()))
        {
            const source::Offset beginOffset = cursor;
            while (!isEof() && isIdentifier(getCurrentCharacter())) advanceCursor();
            const auto lexeme = buffer.contents.substr(beginOffset, cursor - beginOffset);
            return {token::TokenKind::IDENTIFIER, lexeme, {beginLocation, {buffer.id, cursor}}};
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
                context.getDiagnosticEmitter().error(
                    {beginLocation, {buffer.id, cursor}},
                    diagnostics::DiagnosticId::LEXER_UNEXPECTED_CHARACTER, '-'
                );
            }
            const source::Offset beginOffset = cursor;
            advanceCursor(); // Advance to consume possible '-'.
            while (!isEof() && std::isdigit(getCurrentCharacter())) advanceCursor();
            if (getCurrentCharacter() == '.')
            {
                advanceCursor(); // Consume '.'
                while (!isEof() && std::isdigit(getCurrentCharacter())) advanceCursor();
            }
            const auto lexeme = buffer.contents.substr(beginOffset, cursor - beginOffset);
            return {token::TokenKind::NUMBER, lexeme, {beginLocation, {buffer.id, cursor}}};
        }

        // String literal.
        if (getCurrentCharacter() == '"')
        {
            const source::Offset beginOffset = cursor;
            advanceCursor(); // Consume '"'
            while (!isEof() && getCurrentCharacter() != '"') advanceCursor();
            if (getCurrentCharacter() != '"')
            {
                context.getDiagnosticEmitter().error(
                    {beginLocation, {buffer.id, cursor}},
                    diagnostics::DiagnosticId::LEXER_UNBALANCED_QUOTE
                );
            }
            advanceCursor(); // Consume '"'
            // +1 and -1 for excluding the quotes.
            const auto lexeme = buffer.contents.substr(beginOffset + 1, cursor - beginOffset - 2);
            return {token::TokenKind::STRING, lexeme, {beginLocation, {buffer.id, cursor}}};
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
                    return {symbolToTokenKind(symbol), lexeme, {beginLocation, {buffer.id, cursor}}};
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
                                return {token::TokenKind::ARROW, "->", {beginLocation, {buffer.id, cursor}}};
                            }
                        default:
                            break;
                    }
                }
            default:
                break;
        }
        context.getDiagnosticEmitter().error(
            {beginLocation, {buffer.id, cursor}},
            diagnostics::DiagnosticId::LEXER_UNEXPECTED_CHARACTER, getCurrentCharacter()
        );
    }

    char Lexer::getCurrentCharacter() const noexcept { return isEof() ? '\0' : buffer[cursor]; }

    char Lexer::peekNextCurrentCharacter() const noexcept
    {
        return cursor + 1 < buffer.getSize() ? buffer[cursor + 1] : '\0';
    }

    bool Lexer::isEof() const noexcept { return cursor >= buffer.getSize(); }

    void Lexer::advanceCursor() noexcept
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

    token::TokenKind Lexer::symbolToTokenKind(const char symbol) const noexcept
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
                const source::Location location{buffer.id, cursor};
                context.getDiagnosticEmitter().error(
                    {location, location},
                    diagnostics::DiagnosticId::LEXER_UNEXPECTED_CHARACTER, symbol
                );
        }
    }
}
