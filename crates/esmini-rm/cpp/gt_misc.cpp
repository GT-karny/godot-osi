// GTRM_* shim: elevation / super-elevation / lane-offset profiles, per-road
// type/rule/speed/width queries, and network-level metadata (version, speed
// unit, friction, geo offset) — none exposed by the stock esminiRMLib C API.
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

using namespace roadmanager;

// One elevation / super-elevation profile entry: a cubic a..d valid from `s`
// over `length` meters (poly evaluated in the local ds from `s`).
struct GTRM_Elevation
{
    unsigned int road_id;
    double       s;
    double       length;
    double       a;
    double       b;
    double       c;
    double       d;
};

// OpenDRIVE <geoReference> offset (OSI 3.7.0).
struct GTRM_GeoOffset
{
    double x;
    double y;
    double z;
    double hdg;
};

// Network-level metadata. `speed_unit` is SpeedUnit (0 undefined,1 km/h,2 m/s,3 mph).
struct GTRM_NetworkInfo
{
    int    version_major;
    int    version_minor;
    int    speed_unit;
    double friction;
};

namespace
{
    void fill_elevation(unsigned int road_id, Elevation* e, GTRM_Elevation* out)
    {
        out->road_id = road_id;
        out->s       = e->GetS();
        out->length  = e->GetLength();
        out->a       = e->poly3_.GetA();
        out->b       = e->poly3_.GetB();
        out->c       = e->poly3_.GetC();
        out->d       = e->poly3_.GetD();
    }
}  // namespace

extern "C"
{
    // Number of elevation profile entries on `road_id`; -1 on error.
    int GTRM_GetNumberOfElevations(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        return road == nullptr ? -1 : static_cast<int>(road->GetNumberOfElevations());
    }

    // Fill `out` with elevation entry `idx`. 0 / -1.
    int GTRM_GetElevation(unsigned int road_id, unsigned int idx, GTRM_Elevation* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || idx >= road->GetNumberOfElevations())
        {
            return -1;
        }
        Elevation* e = road->GetElevation(idx);
        if (e == nullptr)
        {
            return -1;
        }
        fill_elevation(road_id, e, out);
        return 0;
    }

    // Number of super-elevation (cross slope) entries on `road_id`; -1 on error.
    int GTRM_GetNumberOfSuperElevations(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        return road == nullptr ? -1 : static_cast<int>(road->GetNumberOfSuperElevations());
    }

    // Fill `out` with super-elevation entry `idx`. 0 / -1.
    int GTRM_GetSuperElevation(unsigned int road_id, unsigned int idx, GTRM_Elevation* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || idx >= road->GetNumberOfSuperElevations())
        {
            return -1;
        }
        Elevation* e = road->GetSuperElevation(idx);
        if (e == nullptr)
        {
            return -1;
        }
        fill_elevation(road_id, e, out);
        return 0;
    }

    // Lane-offset (lateral shift of the reference line) at road `s`, into `out`. 0 / -1.
    int GTRM_GetLaneOffset(unsigned int road_id, double s, double* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        *out = road->GetLaneOffset(s);
        return 0;
    }

    // Road traffic rule: 0 = right-hand traffic, 1 = left-hand traffic; -1 on error.
    int GTRM_GetRoadRule(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        return static_cast<int>(road->GetRule());
    }

    // OpenDRIVE road type (roadmanager::Road::RoadType) at road `s`; -1 on error.
    int GTRM_GetRoadType(unsigned int road_id, double s)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        return static_cast<int>(road->GetRoadTypeByS(s));
    }

    // Speed (m/s) from the road type element active at `s`, into `out`. 0 / -1.
    int GTRM_GetRoadSpeed(unsigned int road_id, double s, double* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        *out = road->GetSpeedByS(s);
        return 0;
    }

    // Width (m) of `road_id` at `s` on `side` (-1 right, 1 left, 0 both), into
    // `out`, over any lane type. 0 / -1.
    int GTRM_GetRoadWidth(unsigned int road_id, double s, int side, double* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        *out = road->GetWidth(s, side);
        return 0;
    }

    // Fill `out` with network metadata. 0 / -1 (no network).
    int GTRM_GetNetworkInfo(GTRM_NetworkInfo* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        out->version_major = odr->GetVersionMajor();
        out->version_minor = odr->GetVersionMinor();
        out->speed_unit    = static_cast<int>(odr->GetSpeedUnit());
        out->friction      = odr->GetFriction();
        return 0;
    }

    // Fill `out` with the network geo offset (OSI 3.7.0). 0 / -1.
    int GTRM_GetGeoOffset(GTRM_GeoOffset* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        const GeoOffset& go = odr->GetGeoOffset();
        out->x   = go.x_;
        out->y   = go.y_;
        out->z   = go.z_;
        out->hdg = go.hdg_;
        return 0;
    }
}
