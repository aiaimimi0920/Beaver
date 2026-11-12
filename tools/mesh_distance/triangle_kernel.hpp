#pragma once
#include <algorithm>
#include <array>
#include <cmath>
#include <limits>

namespace exact_bvh {
using Vec = std::array<double, 3>;
// GCC binary128 fallback is supplied by the existing compiler runtime.
// This hit kernel has no binary128 transcendental calls. The separate numeric
// domain validator uses existing libquadmath fmaq for exact zero-area checks.
using Scalar = __float128;
using Wide = std::array<Scalar, 3>;
struct Triangle { Vec a, b, c; };
struct Hit { double distance; Vec point; };

template <class V> V minus(const V &a, const V &b) {
    return {a[0]-b[0], a[1]-b[1], a[2]-b[2]};
}
template <class V> auto dot(const V &a, const V &b) {
    return (a[0]*b[0] + a[1]*b[1]) + a[2]*b[2];
}
template <class V> V cross(const V &a, const V &b) {
    return {a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2],
            a[0]*b[1]-a[1]*b[0]};
}
inline Wide wide(const Vec &v) { return {v[0], v[1], v[2]}; }
inline Vec narrow(const Wide &v) {
    return {static_cast<double>(v[0]), static_cast<double>(v[1]),
            static_cast<double>(v[2])};
}
inline double hit_distance(const Vec &p, const Vec &h) {
    Vec d = minus(h, p);
    double s = dot(d, d);
    if (std::isfinite(s) && s >= std::numeric_limits<double>::min())
        return std::sqrt(s);
    Wide w = minus(wide(h), wide(p));
    return static_cast<double>(std::sqrt(static_cast<long double>(dot(w, w))));
}

// Ordinary triangles retain the legacy Gram formula with stable cyclic origin
// and near-endpoint edge reconstruction. A binary128 cross-product branch
// handles inexact binary64 input differences and cancellation slivers.
// The 0.01 Gram-condition guard selects wider arithmetic, NOT a geometric
// degeneracy tolerance. The build-time numeric_domain guard rejects unsupported
// conditioning and proves represented zero area before admitting degenerates.
inline Hit robust_hit(const Triangle &t, const Vec &p) {
    // Rebase cyclically to the closest input vertex. A distant fixed origin
    // can destroy a small local feature in cross/plane cancellation even when
    // every coordinate subtraction is exact in binary128.
    std::array<Vec, 3> corners{t.a, t.b, t.c};
    Wide x = wide(p);
    int origin = 0;
    Scalar nearest = dot(minus(wide(corners[0]), x), minus(wide(corners[0]), x));
    for (int k = 1; k < 3; ++k) {
        Wide offset = minus(wide(corners[k]), x);
        Scalar squared = dot(offset, offset);
        if (squared < nearest) { nearest = squared; origin = k; }
    }
    Wide a = wide(corners[origin]), b = wide(corners[(origin + 1) % 3]);
    Wide c = wide(corners[(origin + 2) % 3]);
    Wide ab=minus(b,a), ac=minus(c,a), ap=minus(x,a);
    Wide n=cross(ab,ac);
    Scalar n2=dot(n,n);
    Wide answer=a;
    Scalar best_squared=nearest;
    for (auto pair : {std::array<Wide,2>{a,b}, {b,c}, {c,a}}) {
        // Construct a segment hit from its nearer endpoint, never huge+tiny
        // cancellation at a distant endpoint when the local endpoint is near.
        Wide first = minus(x, pair[0]), second = minus(x, pair[1]);
        if (dot(second, second) < dot(first, first)) std::swap(pair[0], pair[1]);
        Wide e=minus(pair[1],pair[0]), q=minus(x,pair[0]);
        Scalar e2=dot(e,e);
        Scalar u=e2 ? std::clamp(dot(q,e)/e2,Scalar(0),Scalar(1)) : 0;
        Wide h;
        for (int k=0;k<3;++k) h[k]=pair[0][k]+u*e[k];
        Scalar squared=dot(minus(h,x),minus(h,x));
        if (squared < best_squared) { best_squared=squared; answer=h; }
    }
    if (n2>0) {
        Scalar u=dot(cross(ap,ac),n)/n2;
        Scalar v=dot(cross(ab,ap),n)/n2;
        if (u>=0 && v>=0 && u+v<=1) {
            Wide h;
            Scalar height=dot(ap,n)/n2;
            for (int k=0;k<3;++k) h[k]=x[k]-height*n[k];
            Scalar squared=dot(minus(h,x),minus(h,x));
            // Compare all geometric candidates before rounding. A rounded hit
            // can move slightly off the plane and spuriously beat a closer
            // on-triangle candidate. The plane still cannot blindly win.
            if (squared < best_squared) { best_squared=squared; answer=h; }
        }
    }
    Vec rounded=narrow(answer);
    return {hit_distance(p,rounded),rounded};
}
// Error-free subtraction residual: reject the binary64 fast path if a small
// input feature was discarded. The admitted domain makes the fallback's
// binary128 input differences exact. This is not a tolerance comparison.
inline bool exact_difference(const Vec &a, const Vec &b, const Vec &difference) {
    for (int k = 0; k < 3; ++k) {
        if (!std::isfinite(difference[k])) return false;
        double virtual_b = a[k] - difference[k];
        double residual = (a[k] - (difference[k] + virtual_b)) + (virtual_b - b[k]);
        if (residual != 0) return false;
    }
    return true;
}
inline Hit local_triangle_hit(const Triangle &t, const Vec &p) {
    Vec ab=minus(t.b,t.a), ac=minus(t.c,t.a), ap=minus(p,t.a);
    if (!exact_difference(t.b, t.a, ab) || !exact_difference(t.c, t.a, ac)
            || !exact_difference(p, t.a, ap))
        return robust_hit(t,p);
    double aa=dot(ab,ab), bb=dot(ab,ac), cc=dot(ac,ac);
    double da=dot(ap,ab), dc=dot(ap,ac), den=aa*cc-bb*bb;
    // Normalize by the largest diagonal, not aa*cc: the latter only detects
    // nearly parallel edges and misses an orthogonal but extremely short edge.
    double diagonal=std::max(aa,cc);
    if (!std::isfinite(den) || den <= 0.01*diagonal*diagonal)
        return robust_hit(t,p);
    double u=(da*cc-dc*bb)/den, v=(dc*aa-da*bb)/den;
    // Even the nearest vertex can be distant from an interior query. Binary64
    // barycentric reconstruction can then cancel away the local hit. Recheck
    // interior/edges and reconstruct by the wide plane projection instead.
    if (u>=0 && v>=0 && u+v<=1) return robust_hit(t,p);
    Hit best{std::numeric_limits<double>::infinity(),t.a};
    for (auto pair : {std::array<Vec,2>{t.a,t.b}, {t.b,t.c}, {t.c,t.a}}) {
        Vec first = minus(p, pair[0]), second = minus(p, pair[1]);
        if (dot(second, second) < dot(first, first)) std::swap(pair[0], pair[1]);
        Vec e=minus(pair[1],pair[0]), q=minus(p,pair[0]);
        double e2=dot(e,e);
        double r=std::clamp(dot(q,e)/e2,0.0,1.0);
        Vec h;
        for (int k=0;k<3;++k) h[k]=pair[0][k]+r*e[k];
        double d=hit_distance(p,h);
        if (d<best.distance) best={d,h};
    }
    return std::isfinite(best.distance) ? best : robust_hit(t,p);
}
inline Hit triangle_hit(const Triangle &t, const Vec &p) {
    // Even a well-conditioned Gram system loses a nearby feature if its
    // reconstruction starts at a distant vertex. Rebase the fast path too.
    std::array<Vec, 3> corners{t.a, t.b, t.c};
    int origin = 0;
    double nearest = std::numeric_limits<double>::infinity();
    for (int k = 0; k < 3; ++k) {
        Vec offset = minus(corners[k], p);
        double squared = dot(offset, offset);
        if (!std::isfinite(squared)) return robust_hit(t, p);
        if (squared < nearest) { nearest = squared; origin = k; }
    }
    Triangle local{corners[origin], corners[(origin + 1) % 3], corners[(origin + 2) % 3]};
    return local_triangle_hit(local, p);
}
} // namespace exact_bvh
