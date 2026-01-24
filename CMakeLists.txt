cmake_minimum_required(VERSION 4.0)
project(lambda-discipline)

set(CMAKE_CXX_STANDARD 20)

include_directories(
    ${CMAKE_SOURCE_DIR}/include
)

add_library(frontend STATIC
    ${CMAKE_SOURCE_DIR}/src/frontend/location.cpp
    ${CMAKE_SOURCE_DIR}/src/frontend/token.cpp
    ${CMAKE_SOURCE_DIR}/src/frontend/lexer.cpp
    ${CMAKE_SOURCE_DIR}/src/frontend/ast.cpp
    ${CMAKE_SOURCE_DIR}/src/frontend/program.cpp
    ${CMAKE_SOURCE_DIR}/src/frontend/parser.cpp
    ${CMAKE_SOURCE_DIR}/src/frontend/type.cpp
)

add_library(runtime STATIC
    ${CMAKE_SOURCE_DIR}/src/runtime/interpreter.cpp
    ${CMAKE_SOURCE_DIR}/src/runtime/builtins.cpp
    ${CMAKE_SOURCE_DIR}/src/runtime/builtin-modules/builtin_module_core.cpp
    ${CMAKE_SOURCE_DIR}/src/runtime/builtin-modules/builtin_module_list.cpp
    ${CMAKE_SOURCE_DIR}/src/runtime/builtin-modules/builtin_module_io.cpp
)

add_executable(lbd
    ${CMAKE_SOURCE_DIR}/src/main.cpp
    ${CMAKE_SOURCE_DIR}/src/cmd.cpp
    ${CMAKE_SOURCE_DIR}/src/repl.cpp
)

target_link_libraries(lbd PRIVATE frontend runtime -static)

# TODO: Add build tests
# EXAMPLE: add_executable(lexer_test ../tests/lexer_test.cc)
#          target_link_libraries(lexer_test PRIVATE frontend)