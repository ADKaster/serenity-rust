
if (ENABLE_EXPERIMENTAL_RUST)
    include(FetchContent)

    FetchContent_Declare(
      Corrosion
      GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
      GIT_TAG origin/master # Optionally specify a version tag or branch here
    )

    set(Rust_ROOT "${SerenityOS_SOURCE_DIR}/Toolchain/Local/rust")
    set(Rust_CARGO_TARGET "${SERENITY_ARCH}-unknown-serenity")

    FetchContent_MakeAvailable(Corrosion)

endif()

# https://github.com/corrosion-rs/corrosion/issues/172
function(corrosion_link_libraries_fixed target_name)
    add_dependencies(cargo-build_${target_name} ${ARGN})
    foreach(library ${ARGN})
        set_property(
            TARGET cargo-build_${target_name}
            APPEND
            PROPERTY CARGO_DEPS_LINKER_LANGUAGES
            $<TARGET_PROPERTY:${library},LINKER_LANGUAGE>
        )
        corrosion_add_target_rustflags(${target_name} "-L$<TARGET_LINKER_FILE_DIR:${library}>")

        if (TARGET ${library})
            corrosion_add_target_rustflags(${target_name} "-l$<TARGET_PROPERTY:${library},OUTPUT_NAME>")
        else()
            corrosion_add_target_rustflags(${target_name} "-l${library}")
        endif()
    endforeach()
endfunction()

# corrosion_install_targets doesn't install libraries
# https://github.com/corrosion-rs/corrosion/issues/64
# Let's make some assumptions about these crates:
#   - if there's an executable, only install the executable
#   - if there's libraries, there might be both static and shared
#   - don't try to install libs and executables for the same crate
function(serenity_install_rust_target target_name)
  get_property(
    TARGET_TYPE
    TARGET ${target_name} PROPERTY TYPE
  )

  set(PERMS OWNER_WRITE OWNER_READ GROUP_READ WORLD_READ OWNER_EXECUTE GROUP_EXECUTE WORLD_EXECUTE)
 
  if (TARGET_TYPE STREQUAL "EXECUTABLE")
    install(FILES "$<TARGET_FILE:${target_name}>"
            DESTINATION bin
            PERMISSIONS ${PERMS} 
    )
  else()
    if (TARGET "${target_name}-static")
      install(FILES "$<TARGET_FILE:${target_name}-static>"
              DESTINATION usr/lib
              PERMISSIONS ${PERMS}
      )
    endif()
    if (TARGET "${target_name}-shared")
      install(FILES "$<TARGET_FILE:${target_name}-shared>"
              DESTINATION usr/lib
              PERMISSIONS ${PERMS}
      )
    endif()
  endif()

endfunction()

function(serenity_rust_crate crate_name)
  corrosion_import_crate(MANIFEST_PATH "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml")
  corrosion_link_libraries_fixed("${crate_name}" LibC LibM LibPthread)

  serenity_install_rust_target("${crate_name}")
endfunction()
