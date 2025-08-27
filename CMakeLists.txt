cmake_minimum_required(VERSION 4.0)
project(lambda_discipline)

set(CMAKE_CXX_STANDARD 20)

include_directories(
    ${CMAKE_SOURCE_DIR}/include
)

add_library(fe STATIC
    ${CMAKE_SOURCE_DIR}/src/fe/loc.cpp
    ${CMAKE_SOURCE_DIR}/src/fe/token.cpp
    ${CMAKE_SOURCE_DIR}/src/fe/lexer.cpp
    ${CMAKE_SOURCE_DIR}/src/fe/ast.cpp
    ${CMAKE_SOURCE_DIR}/src/fe/parser.cpp
)

add_library(intp STATIC
    ${CMAKE_SOURCE_DIR}/src/intp/types.cpp
)

add_executable(lambda_discipline ${CMAKE_SOURCE_DIR}/src/main.cpp)
target_link_libraries(lambda_discipline PRIVATE fe intp)

# TODO: Add build tests
# EXAMPLE: add_executable(lexer_test ../tests/lexer_test.cc)
#          target_link_libraries(lexer_test PRIVATE fe)