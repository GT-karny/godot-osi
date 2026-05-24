// Shared helpers for the GTRM_* C++ shims (crates/esmini-rm/cpp/gt_*.cpp).
//
// The stock esminiRMLib C API exposes only Position-based point queries. These
// shims reach into RoadManager's public C++ classes to expose the road-network
// structure itself (geometry, OSI points, lanes, objects, junctions, signals,
// ...). We never modify anything under external/esmini (its CLAUDE.md R1: the
// EnvironmentSimulator core stays pristine) — we only consume it, linking the
// RoadManager static lib our build.rs already produces.
//
// Conventions used across the shims:
//   (A) record enumeration : GTRM_GetNumberOfX() + GTRM_GetX(idx, *out_pod)
//   (B) variable geometry   : GTRM_BuildX() -> vertex count, GTRM_CopyX(out...),
//                             GTRM_ClearX() backed by a `gt::TriBuf`.
#pragma once

#include "RoadManager.hpp"
#include "CommonMini.hpp"

#include <algorithm>
#include <cstddef>
#include <vector>

namespace gt
{
    // The single global OpenDrive network esminiRMLib keeps loaded (null if none).
    inline roadmanager::OpenDrive* odr()
    {
        return roadmanager::Position::GetOpenDrive();
    }

    // A flat triangle-soup buffer with one int attribute per vertex (e.g. a
    // RoadMarkColor or lane-type tag), matching the build/copy/clear convention.
    struct TriBuf
    {
        std::vector<double> verts;  // x,y,z per vertex
        std::vector<int>    attr;   // one int per vertex

        void clear()
        {
            verts.clear();
            attr.clear();
        }
        // Free the backing capacity (used by GTRM_ClearX after copy-out).
        void release()
        {
            std::vector<double>().swap(verts);
            std::vector<int>().swap(attr);
        }
        std::size_t vertex_count() const
        {
            return attr.size();
        }
        void push_vertex(const double p[3], int a)
        {
            verts.push_back(p[0]);
            verts.push_back(p[1]);
            verts.push_back(p[2]);
            attr.push_back(a);
        }
        // Two triangles for the quad (a, b, c, d) wound a-b-c, a-c-d.
        void push_quad(const double a[3], const double b[3], const double c[3], const double d[3], int v)
        {
            push_vertex(a, v);
            push_vertex(b, v);
            push_vertex(c, v);
            push_vertex(a, v);
            push_vertex(c, v);
            push_vertex(d, v);
        }
        // Copy out: out_xyz needs 3*vertex_count doubles, out_attr needs
        // vertex_count ints. Either may be null to skip.
        void copy_out(double* out_xyz, int* out_attr) const
        {
            if (out_xyz != nullptr)
            {
                std::copy(verts.begin(), verts.end(), out_xyz);
            }
            if (out_attr != nullptr)
            {
                std::copy(attr.begin(), attr.end(), out_attr);
            }
        }
    };

    // Offset an OSI centerline point laterally by `lateral` meters, perpendicular
    // to the point's orientation, lifting it by `z_off` (as esmini's roadgeom
    // does, via RotateY). Result written to out[3].
    inline void offset_point(const roadmanager::PointStruct& pt, double lateral, double z_off, double out[3])
    {
        double v[3];
        RotateY(lateral, pt.r, pt.p, pt.h, v);
        out[0] = pt.x + v[0];
        out[1] = pt.y + v[1];
        out[2] = pt.z + v[2] + z_off;
    }
}  // namespace gt
