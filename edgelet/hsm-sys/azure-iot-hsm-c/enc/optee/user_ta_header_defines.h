/* Copyright (c) Microsoft Corporation. All rights reserved. */
/* Licensed under the MIT License. */
#pragma once

#define TA_UUID { /* a511a7ab-a1a4-4ba6-bf2a-447c6c1fb116 */ \
    0xa511a7ab, \
    0xa1a4, \
    0x4ba6, \
    {0xbf, 0x2a, 0x44, 0x7c, 0x6c, 0x1f, 0xb1, 0x16} \
  }

#define TA_FLAGS                    (TA_FLAG_MULTI_SESSION | TA_FLAG_EXEC_DDR)
#define TA_STACK_SIZE               (12 * 1024)        /* 12 KB */
#define TA_DATA_SIZE                (1 * 1024 * 1024)  /* 1 MB */

#define TA_CURRENT_TA_EXT_PROPERTIES \
    { "gp.ta.description", USER_TA_PROP_TYPE_STRING, \
        "Azure IoT Edge HSM TA" }, \
    { "gp.ta.version", USER_TA_PROP_TYPE_U32, &(const uint32_t){ 0x0010 } }
