#include "ChaCha20Poly1305.h"
#include "CommonServices.h"

#include <stdint.h>
#include <stddef.h>

typedef int32_t OSStatus;

// Decrypt a frame using ChaCha20-Poly1305 exactly like Apple does
OSStatus chachadecrypt_64x64(
    const uint8_t key[32],       // read-only key
    uint8_t nonce[8],            // nonce gets incremented
    const void *inHeader,        // header slice (AAD)
    uint8_t *inFramePtr,         // ciphertext + tag, decrypted in-place
    uint32_t bodySize            // includes 16-byte tag
) {
    if (!inHeader || !inFramePtr || bodySize < 16) {
        return -1; // invalid args
    }

    OSStatus err = 0;
    size_t len = 0;
    chacha20_poly1305_state state;

    // Init cipher with key + nonce
    chacha20_poly1305_init_64x64(&state, key, nonce);

    // Header is AAD (must match exactly AirPlayScreenHeader)
    chacha20_poly1305_add_aad(&state, inHeader, 128);

    // Decrypt ciphertext (bodySize - 16)
    len = chacha20_poly1305_decrypt(&state, inFramePtr, bodySize - 16, inFramePtr);

    // Verify the 16-byte Poly1305 tag at the end
    len += chacha20_poly1305_verify(
        &state,
        &inFramePtr[len],                    // starting point of tag bytes (could be overlapping ciphertext)
        &inFramePtr[bodySize - 16],          // tag location
        &err
    );

    // Ensure full body decrypted
    if (len != bodySize - 16 || err != 0) {
        return -6754; // OSStatus errSecAuthFailed (Apple tag verification failure)
    }

    // Increment nonce exactly like Apple
    LittleEndianIntegerIncrement(nonce, sizeof(uint8_t) * 8);

    return 0;
}
