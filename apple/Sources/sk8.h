// sk8.h - the C ABI of beargarden/sk8 (sk8/src/ffi.rs), for the Swift app.
// The structs match sk8::input::VirtualController and sk8::frame::Frame (repr(C);
// sk8/src/frame.rs tests their size and offsets).
#pragma once
#include <stdint.h>
#include <stddef.h>

typedef struct Sk8Handle Sk8Handle;

typedef struct {
    float lx, ly, rx, ry, lt, rt;   // sticks [-1, 1] (+y up), triggers [0, 1]
    uint32_t buttons;               // bit i: A B X Y LB RB L3 R3 DUp DDown DLeft DRight
} Sk8Controller;

typedef struct {
    uint64_t tick;
    float pos[3], fwd[3], up[3];
    float flip_angle, shuv_angle, pitch;
    float rider_pos[3], rider_up[3];
    float crouch, lean, body_flip;
    uint32_t clip, goofy, fakie, state;
    float speed;
    float cam_pos[3], cam_target[3];
    float cam_fov;
    float score, line, line_timer, multiplier, sequence;
} Sk8Frame;

Sk8Handle *sk8_new(const uint8_t *json, size_t len, uint32_t difficulty);
void sk8_free(Sk8Handle *h);
void sk8_tune(Sk8Handle *h, uint32_t difficulty, const uint8_t *text, size_t len);
void sk8_set_stance(Sk8Handle *h, uint32_t goofy);
void sk8_set_sample_rate(Sk8Handle *h, float hz);
uint32_t sk8_advance(Sk8Handle *h, float dt, const Sk8Controller *samples, size_t n);
const Sk8Frame *sk8_frame(const Sk8Handle *h);
const Sk8Frame *sk8_prev_frame(const Sk8Handle *h);
float sk8_alpha(const Sk8Handle *h);
size_t sk8_events(Sk8Handle *h, uint8_t *out, size_t cap);
uint32_t sk8_place_at(Sk8Handle *h, const uint8_t *name, size_t len);
size_t sk8_frame_size(void);
