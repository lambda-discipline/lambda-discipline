#pragma once

#include <iostream>
#include <vector>

#if defined(_WIN32)
#include <windows.h>
#endif

inline bool enable_virtual_terminal() {
#if defined(_WIN32)
    const HANDLE output_handle = GetStdHandle(STD_OUTPUT_HANDLE); // NOLINT(*-misplaced-const)
    if (output_handle == INVALID_HANDLE_VALUE) {
        return false;
    }
    DWORD dwMode = 0;
    if (!GetConsoleMode(output_handle, &dwMode)) {
        return false;
    }
    dwMode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    if (!SetConsoleMode(output_handle, dwMode)) {
        return false;
    }
#endif
    return true;
}

namespace colors {
    constexpr auto RESET = "\033[0m";
    constexpr auto RED = "\033[31m";
    constexpr auto GREEN = "\033[32m";
    constexpr auto YELLOW = "\033[33m";
    constexpr auto BLUE = "\033[34m";
    constexpr auto CYAN = "\033[36m";
    constexpr auto WHITE = "\033[37m";
    constexpr auto GREY = "\033[38;5;200m";
}

inline void print_table_row(const std::vector<std::string> &row, const std::vector<size_t> &widths,
                            const std::string &color = "") {
    std::cout << color;
    for (size_t i = 0; i < row.size(); ++i) {
        std::cout << std::setw(static_cast<int>(widths[i])) << std::left << row[i] << "  ";
    }
    std::cout << colors::RESET << std::endl;
}

inline void print_table(const std::vector<std::string> &headers, const std::vector<std::vector<std::string> > &data,
                        const std::string &color = colors::CYAN) {
    std::vector<size_t> widths(headers.size(), 0);
    for (size_t i = 0; i < headers.size(); ++i) {
        widths[i] = headers[i].size();
    }
    for (const auto &row: data) {
        for (size_t i = 0; i < row.size(); ++i) {
            if (static_cast<int>(row[i].size()) > widths[i]) {
                widths[i] = row[i].size();
            }
        }
    }
    print_table_row(headers, widths, color);
    for (size_t i = 0; i < headers.size(); ++i) {
        std::cout << std::string(widths[i], '-') << "  ";
    }
    std::cout << std::endl;
    for (const auto &row: data) {
        print_table_row(row, widths);
    }
}
