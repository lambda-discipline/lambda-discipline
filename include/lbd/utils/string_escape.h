#pragma once
#include <string>

inline std::string escapeString(const std::string &data) {
    std::string result;
    result.reserve(data.size());
    for (const char c: data) {
        switch (c) {
            case '\n': result += "\\n";
                break;
            case '\t': result += "\\t";
                break;
            case '\r': result += "\\r";
                break;
            case '\\': result += "\\\\";
                break;
            case '"': result += "\\\"";
                break;
            case '\'': result += "\\\'";
                break;
            default:
                // Printable ASCII characters.
                if (c >= 32 && c <= 126) {
                    result.push_back(c);
                } else {
                    // Non-printable: use hex escape.
                    char buffer[5];
                    snprintf(buffer, sizeof(buffer), "\\x%02X", static_cast<unsigned char>(c));
                    result += buffer;
                }
        }
    }
    return result;
}

inline std::string unescapeString(const std::string &data) {
    std::string result;
    result.reserve(data.size());
    for (size_t index = 0; index < data.size(); ++index) {
        if (data[index] == '\\' && index + 1 < data.size()) {
            switch (data[index + 1]) {
                case 'n': result.push_back('\n');
                    break;
                case 't': result.push_back('\t');
                    break;
                case 'r': result.push_back('\r');
                    break;
                case '\\': result.push_back('\\');
                    break;
                case '"': result.push_back('"');
                    break;
                default:
                    result.push_back(data[index + 1]); // Unknown escape: take character.
            }
            ++index; // Skip the escaped char.
        } else {
            result.push_back(data[index]);
        }
    }
    return result;
}
