use super::AppErr;
use crate::{
    config::Config,
    tiles::{self, Marker},
};
use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

pub async fn tile_path(z: u8, x: u32, y: u32, create: bool) -> Option<PathBuf> {
    let mut path =
        Path::new(Config::TILES_DIR).join(z.to_string()).join(x.to_string());

    if create {
        let _ = tokio::fs::create_dir_all(&path).await;
        path.push(y.to_string());
        path.set_extension("pbf");
        return Some(path);
    }

    if !path.is_dir() {
        return None;
    }

    path.push(y.to_string());
    path.set_extension("pbf");

    if path.is_file() {
        return Some(path);
    }

    None
}

pub async fn tile_file(path: PathBuf) -> Result<tokio::fs::File, AppErr> {
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        // .custom_flags(libc::O_ASYNC)
        .open(path)
        .await?;

    Ok(file)
}

pub async fn marker_set(mut marker: Marker) -> Result<(), AppErr> {
    let gene = marker.gene;
    let base_zoom = marker.point().zom;

    for zoom in base_zoom..=Config::MAX_ZOOM {
        marker.set_zoom(zoom);
        let p = marker.point();
        let (col, row) = p.to_col_row();
        // let db_row = (1 << p.zom) - 1 - row;

        let mut file =
            tile_file(tile_path(zoom, col, row, true).await.unwrap()).await?;
        let mut file_data = Vec::<u8>::new();
        file.seek(SeekFrom::Start(0)).await?;
        let file_size = file.read_to_end(&mut file_data).await?;

        if file_size == 0 {
            let tile_data = tiles::encode(vec![marker.clone()])?;
            file.write_all(&tile_data).await?;
            continue;
        }

        file.seek(SeekFrom::Start(0)).await?;

        let Ok(mut markers) = tiles::decode(file_data) else {
            log::warn!("marker_set tile decode failed");
            let tile_data = tiles::encode(vec![marker.clone()])?;
            file.write_all(&tile_data).await?;
            file.set_len(tile_data.len() as u64).await?;
            continue;
        };

        let mut changed = false;
        for m in markers.iter_mut() {
            if m.gene == gene {
                m.clone_from(&marker);
                changed = true;
            }
        }
        if !changed {
            markers.push(marker.clone());
        }

        let tile_data = tiles::encode(markers)?;
        file.write_all(&tile_data).await?;
        file.set_len(tile_data.len() as u64).await?;
    }

    Ok(())
}

pub async fn marker_del(marker: Marker) -> Result<(), AppErr> {
    let gene = marker.gene;
    let point = marker.point();

    for zoom in point.zom..=Config::MAX_ZOOM {
        let p = point.with_zoom(zoom);
        let (col, row) = p.to_col_row();
        // let db_row = (1 << p.zom) - 1 - row;

        let Some(path) = tile_path(zoom, col, row, false).await else {
            continue;
        };
        let mut file = tile_file(path).await?;

        let mut file_data = Vec::<u8>::new();
        let file_size = file.read_to_end(&mut file_data).await?;

        if file_size == 0 {
            continue;
        }

        let Ok(mut markers) = tiles::decode(file_data) else {
            log::warn!("marker_del tile decode error");
            continue;
        };

        markers.retain(|m| m.gene != gene);
        let tile_data = tiles::encode(markers)?;

        file.seek(SeekFrom::Start(0)).await?;
        file.write_all(&tile_data).await?;
        file.set_len(tile_data.len() as u64).await?;
    }

    Ok(())
}
