#pragma once
#include "triangle_kernel.hpp"
#include <cstdint>
#include <numeric>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace exact_bvh {
constexpr double INF=std::numeric_limits<double>::infinity();
inline double down(double x) { return std::max(0.0,std::nextafter(x,-INF)); }
inline double up(double x) { return std::nextafter(x,INF); }
struct Node {
    Vec low, high;
    int64_t start=0, count=0, left=-1, right=-1;
};
struct Index {
    std::vector<Triangle> triangles;
    std::vector<int64_t> order;
    std::vector<Node> nodes;
    int64_t build(int64_t start,int64_t count) {
        Node n;
        n.low={INF,INF,INF}; n.high={-INF,-INF,-INF};
        n.start=start; n.count=count;
        for (int64_t j=start;j<start+count;++j) {
            auto &t=triangles[order[j]];
            for (const Vec &p : {t.a,t.b,t.c}) for (int k=0;k<3;++k) {
                n.low[k]=std::min(n.low[k],p[k]);
                n.high[k]=std::max(n.high[k],p[k]);
            }
        }
        // Outward ulp inflation plus directed arithmetic below: lower bound
        // never rounds above the exact Euclidean point-to-box distance.
        for (int k=0;k<3;++k) {
            n.low[k]=std::nextafter(n.low[k],-INF);
            n.high[k]=std::nextafter(n.high[k],INF);
        }
        int64_t id=nodes.size(); nodes.push_back(n);
        if (count>8) {
            int axis=0;
            for (int k=1;k<3;++k)
                if ((long double)n.high[k]-n.low[k] > (long double)n.high[axis]-n.low[axis]) axis=k;
            int64_t middle=start+count/2;
            auto center=[&](int64_t i) {
                auto &t=triangles[i];
                return ((long double)t.a[axis]+t.b[axis]+t.c[axis])/3;
            };
            std::nth_element(order.begin()+start,order.begin()+middle,
                             order.begin()+start+count,[&](int64_t a,int64_t b) {
                auto ca=center(a), cb=center(b);
                return ca<cb || (ca==cb && a<b);
            });
            nodes[id].left=build(start,middle-start);
            nodes[id].right=build(middle,start+count-middle);
            nodes[id].count=0;
        }
        return id;
    }
};
inline double box_lower(const Node &n,const Vec &p) {
    double sum=0;
    for (int k=0;k<3;++k) {
        double d=0;
        if (p[k]<n.low[k]) d=down(n.low[k]-p[k]);
        if (p[k]>n.high[k]) d=down(p[k]-n.high[k]);
        sum=down(sum+down(d*d));
    }
    return sum;
}
inline double vertex_upper(const Vec &a,const Vec &p) {
    double sum=0;
    for (int k=0;k<3;++k) {
        double d=up(std::abs(a[k]-p[k]));
        sum=up(sum+up(d*d));
    }
    return sum;
}
struct Result { double distance=INF; int64_t id=-1; Vec hit{}; uint64_t tested=0; };
Result query(const Index &mesh,const Vec &p,bool brute) {
    Result result;
    double bound=vertex_upper(mesh.triangles[0].a,p);
    auto test=[&](int64_t i) {
        const auto &tri=mesh.triangles[i];
        Hit hit=triangle_hit(tri,p);
        ++result.tested;
        if (hit.distance<result.distance || (hit.distance==result.distance && (result.id<0 || i<result.id))) {
            result.distance=hit.distance; result.id=i; result.hit=hit.point;
        }
        // Only a genuine input vertex supplies pruning bounds. Rounded
        // projections are NOT guaranteed to lie exactly on the input surface.
        bound=std::min({bound,vertex_upper(tri.a,p),vertex_upper(tri.b,p),vertex_upper(tri.c,p)});
    };
    if (brute) {
        for (size_t i=0;i<mesh.triangles.size();++i) test(i);
        return result;
    }
    std::array<int64_t,128> stack{};
    size_t size=0; stack[size++]=0;
    while (size) {
        auto &node=mesh.nodes[stack[--size]];
        if (box_lower(node,p)>bound) continue;
        if (node.count) {
            for (int64_t i=node.start;i<node.start+node.count;++i) test(mesh.order[i]);
        } else {
            int64_t near=node.left,far=node.right;
            if (box_lower(mesh.nodes[far],p)<box_lower(mesh.nodes[near],p)) std::swap(near,far);
            if (size+2>stack.size()) throw std::runtime_error("BVH depth limit");
            stack[size++]=far; stack[size++]=near;
        }
    }
    return result;
}
}
