# the minimum version of CMake.
cmake_minimum_required(VERSION 3.5.0)
project(vfs_apis)

set(NATIVERENDER_ROOT_PATH ${CMAKE_CURRENT_SOURCE_DIR})

if(DEFINED PACKAGE_FIND_FILE)
    include(${PACKAGE_FIND_FILE})
endif()

include_directories(${NATIVERENDER_ROOT_PATH}
                    ${NATIVERENDER_ROOT_PATH}/include)

set(OHOS_SDK_ROOT "/Applications/DevEco-Studio.app/Contents/sdk/default")
set(HMS_LIB_DIR "${OHOS_SDK_ROOT}/hms/native/sysroot/usr/lib/aarch64-linux-ohos")

add_library(entry SHARED napi_init.cpp)
target_link_directories(entry PRIVATE 
    ${HMS_LIB_DIR}
    ${NATIVERENDER_ROOT_PATH}/../../../libs/arm64-v8a
)
target_link_libraries(entry PUBLIC 
    libace_napi.z.so
    libhilog_ndk.z.so
    librcp_c.so
    libvfs_apis.so
)
