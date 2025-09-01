#include <lbd/cmd.h>

namespace cmd {
    void print_help(std::ostream &os, const std::string &program_name) {
        os << "usage: " << program_name << " [options]\n\n"
                << "options:\n"
                << "  -f, --file <filepath>   Specify input source filepath to run\n"
                << "  -h, --help              Show this help message and exit\n"
                << "  -d, --debug             Enable debug mode\n"
                << "  -r, --repl              Run in interactive REPL node" << std::endl;
    }

    Options parse_args(const int argc, char **argv, const std::string &program_name) {
        Options opts;
        for (int i = 1; i < argc; ++i) {
            if (std::string arg = argv[i]; arg == "-h" || arg == "--help") {
                opts.show_help = true;
            } else if (arg == "-f" || arg == "--file") {
                if (i + 1 < argc) {
                    opts.filepath = argv[++i];
                } else {
                    std::cerr << "error: missing filepath after " << arg << std::endl;
                    print_help(std::cerr, program_name);
                    std::exit(EXIT_FAILURE);
                }
            } else if (arg == "-d" || arg == "--debug") {
                opts.debug = true;
            } else if (arg == "-r" || arg == "--repl") {
                opts.repl = true;
            } else {
                std::cerr << "unknown option: " << arg << "\n";
                print_help(std::cerr, program_name);
                std::exit(EXIT_FAILURE);
            }
        }
        return opts;
    }
}
