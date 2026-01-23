#include <lbd/frontend/lexer.h>
#include <lbd/logs.h>
#include <sstream>
#include <utility>
#include <fstream>

namespace frontend {
    static global::Options optionsValue;

    char Lexer::peek() const {
        return position < source.size() ? source[position] : '\0';
    }

    char Lexer::get() {
        const char c = peek();
        if (c != '\0') {
            ++position;
            if (c == '\n') {
                ++row;
                col = 1;
            } else {
                ++col;
            }
        }
        return c;
    }

    bool Lexer::isEof() const {
        return position >= source.size();
    }

    Location Lexer::getCurrentLocation() const {
        return {row, col, filepath};
    }

    Lexer::Lexer(const std::string &filepath, FromFile, global::Options options_) : filepath(filepath) {
        std::ifstream ifs(filepath);
        optionsValue = options_;
        if (!ifs) {
            optionsValue.logger.error({}, "IO error: could not open file ", filepath);
        }
        std::ostringstream ss;
        ss << ifs.rdbuf();
        source = ss.str();
    }

    Lexer::Lexer(std::string str, FromRepl, const global::Options options_) : source(std::move(str)) {
        optionsValue = options_;
    }

    token::Token Lexer::nextToken() {
        char currentCharacter = peek();
        // Skip whitespace.
        while (std::isspace(currentCharacter)) {
            get(); // Consume ' '.
            currentCharacter = peek();
        }
        const Location currentLocation = getCurrentLocation();
        if (isEof()) {
            return {token::Eof(), currentLocation};
        }
        // Identifiers [a-zA-Z_][a-zA-Z0-9_]*
        if (std::isalpha(currentCharacter) || currentCharacter == '_') {
            const size_t start = position;
            while (std::isalnum(currentCharacter) || currentCharacter == '_') {
                get();
                currentCharacter = peek();
            }
            const std::string value = source.substr(start, position - start);
            return {token::Identifier{value}, currentLocation};
        }
        auto lexFloat = [this, &currentCharacter]() -> double {
            const size_t start = position;
            while (std::isdigit(currentCharacter)) {
                get();
                currentCharacter = peek();
            }
            if (currentCharacter == '.') {
                get(); // Consume '.'.
                currentCharacter = peek();
                while (std::isdigit(currentCharacter)) {
                    get();
                    currentCharacter = peek();
                }
            }
            const double value = std::stod(source.substr(start, position - start));
            return value;
        };
        // Positive Float.
        if (std::isdigit(currentCharacter)) {
            return {token::Float(lexFloat()), currentLocation};
        }
        // String Literal: " ... "
        if (currentCharacter == '"') {
            get(); // Consume '"'.
            const size_t start = position;
            currentCharacter = peek();
            while (currentCharacter != '"' && !isEof()) {
                get();
                currentCharacter = peek();
            }
            const std::string value = source.substr(start, position - start);
            if (currentCharacter != '"') {
                optionsValue.logger.error(currentLocation, "syntax error: unbalanced quote");
            }
            get(); // Consume '"'.
            return {token::String{value}, currentLocation};
        }
        // Symbols.
        switch (currentCharacter) {
            case ':':
                get();
                return {token::Colon{}, currentLocation};
            case '=':
                get();
                return {token::Equal{}, currentLocation};
            case '\\':
                get();
                return {token::BackwardSlash{}, currentLocation};
            case '.':
                get();
                return {token::Dot{}, currentLocation};
            case '(':
                get();
                return {token::OpenParenthesis{}, currentLocation};
            case ')':
                get();
                return {token::CloseParenthesis{}, currentLocation};
            case '-':
                get();
                currentCharacter = peek();
                if (currentCharacter == '>') {
                    get();
                    return {token::Arrow{}, currentLocation};
                }
                if (currentCharacter == '-') {
                    // Comment
                    while (currentCharacter != '\n' && !isEof()) {
                        currentCharacter = get();
                    }
                    // After skipping comment, return the next token
                    return nextToken();
                }
                // Negative Float
                if (std::isdigit(currentCharacter)) {
                    return {token::Float(-lexFloat()), currentLocation};
                }
                break;
            default:
                break;
        }
        optionsValue.logger.error(currentLocation, "syntax error: unexpected character ", currentCharacter);
    }

    std::vector<token::Token> Lexer::lexAll() {
        std::vector<token::Token> tokens;
        while (true) {
            token::Token token = nextToken();
            tokens.push_back(token);
            if (std::holds_alternative<token::Eof>(token.tokenType)) {
                break;
            }
        }
        return tokens;
    }
}
