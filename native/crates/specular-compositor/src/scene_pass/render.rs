//! [`Compositor::render_scene`]: one multisampled pass for a whole scene.

use std::time::Instant;

use specular_core::PageId;
use specular_doc::EntityId;
use specular_scene::{Draw, Scene, Size};

use super::batch::{Batch, batch};
use super::build::{DrawOp, Op, Output, QuadTexture, build};
use super::place::{PlaceCounts, Prim, Scissor, ViewTransform, place};
use super::target::MultisampledTarget;
use super::text::{TextDraw, TextItem, TextSystem};
use super::{FrameView, ScenePass, SceneStats};
use crate::Compositor;
use crate::draw_list::DrawCounts;
use crate::gpu_types::FrameUniforms;
use crate::grid::grid_metrics;
use crate::layers::PageLayers;

/// What preparing a frame found.
struct Prepared {
    pages: DrawCounts,
    placed: PlaceCounts,
    items: u32,
    batches: u32,
    text_batches: u32,
}

impl Compositor {
    /// Draws `scene` over the grid into `target` and submits the work.
    ///
    /// Items are painted in list order, pages included, so an item after a
    /// page is over it and an item before it is under. `page_of` maps a page
    /// entity to the page host whose frames it shows; a page with no host or
    /// no frame yet is counted in
    /// [`pages_without_texture`](crate::RenderStats::pages_without_texture)
    /// and left out.
    ///
    /// The pass is 4x multisampled and resolves into `target`, which may be
    /// any size and is taken to be `frame.viewport * frame.scale_factor`.
    pub fn render_scene(
        &mut self,
        target: &wgpu::TextureView,
        frame: &FrameView,
        scene: &Scene,
        page_of: impl Fn(&EntityId) -> Option<PageId>,
    ) -> SceneStats {
        let started = Instant::now();
        self.reclaim();
        let size = target.texture().size();
        if size.width == 0 || size.height == 0 {
            return SceneStats::default();
        }
        let view = ViewTransform {
            camera: frame.camera,
            viewport: frame.viewport,
            scale_factor: frame.scale_factor,
            target: [size.width, size.height],
        };
        let prepared = self.prepare_scene(&view, frame.zooming, scene, &page_of);
        let max_paint_to_submit = self.mark_shown(started);

        let grid = grid_metrics(&frame.camera, &frame.grid, frame.scale_factor);
        let uniforms = FrameUniforms::new(
            &frame.camera,
            frame.viewport,
            frame.scale_factor,
            &frame.grid,
            &grid,
            !self.target_format.is_srgb(),
        );
        self.queue
            .write_buffer(&self.frame_uniforms, 0, bytemuck::bytes_of(&uniforms));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("specular-scene-frame"),
            });
        self.encode_scene_pass(&mut encoder, target, size);
        self.submit(encoder);
        if let Some(text) = &mut self.scene_pass.text {
            text.end_frame();
        }

        let mut render = self.take_frame_stats(max_paint_to_submit);
        render.pages_without_texture = prepared.pages.pages_without_texture;
        render.cpu_textures = prepared.pages.cpu_textures;
        SceneStats {
            render,
            items_drawn: prepared.items,
            items_culled: prepared.placed.culled,
            text_runs_too_small: prepared.placed.text_too_small,
            batches: prepared.batches,
            text_batches: prepared.text_batches,
        }
    }

    /// Culls, batches and builds the frame, and uploads its buffers.
    fn prepare_scene(
        &mut self,
        view: &ViewTransform,
        zooming: bool,
        scene: &Scene,
        page_of: &impl Fn(&EntityId) -> Option<PageId>,
    ) -> Prepared {
        let Self {
            device,
            queue,
            target_format,
            pages,
            instances,
            shape_instances,
            draw_items,
            instance_buffer,
            shape_buffer,
            scene_pass,
            ..
        } = self;
        let ScenePass {
            placed,
            draws,
            mesher,
            mesh,
            mesh_vertices,
            mesh_indices,
            text,
            images,
            ..
        } = scene_pass;

        let has_text = scene
            .items
            .iter()
            .any(|item| matches!(item.draw, Draw::Text(_) | Draw::Column(_)));
        if has_text && text.is_none() {
            *text = Some(TextSystem::new(device, queue, *target_format));
        }
        if let Some(text) = text {
            text.begin_frame(device, queue, view, zooming);
        }

        let place_counts = place(
            scene,
            view,
            |run| {
                text.as_mut()
                    .map_or(Size::default(), |text| text.measure(run))
            },
            placed,
        );
        let batches = batch(placed, view);
        let page_counts = build(
            scene,
            placed,
            &batches,
            view,
            |entity| {
                let id = page_of(entity)?;
                Some((id, pages.get(&id).and_then(PageLayers::info)?))
            },
            |image| images.contains_key(&image),
            &mut Output {
                quads: instances,
                shapes: shape_instances,
                mesher,
                mesh,
                page_layers: draw_items,
                draws,
            },
        );

        let text_batches = batches.iter().filter_map(|batch| match batch.prim {
            Prim::Text(space) => Some((space, batch)),
            Prim::Page | Prim::Image | Prim::Shape | Prim::Mesh => None,
        });
        let mut text_batch_count = 0;
        if let Some(text) = text {
            for (slot, (space, batch)) in text_batches.enumerate() {
                let runs = text_draws(scene, placed, batch);
                text.prepare(device, queue, view, slot, space, runs);
                text_batch_count += 1;
            }
        }

        instance_buffer.write(device, queue, instances);
        shape_buffer.write(device, queue, shape_instances);
        mesh_vertices.write(device, queue, &mesh.vertices);
        mesh_indices.write(device, queue, &mesh.indices);
        Prepared {
            pages: page_counts,
            placed: place_counts,
            items: placed.len() as u32,
            batches: batches.len() as u32,
            text_batches: text_batch_count,
        }
    }

    fn encode_scene_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: wgpu::Extent3d,
    ) {
        let ScenePass {
            draws,
            mesh_vertices,
            mesh_indices,
            text,
            multisampled,
            images,
            ..
        } = &mut self.scene_pass;
        let attachment =
            MultisampledTarget::for_size(multisampled, &self.device, self.target_format, size);
        let pipelines = &self.pipelines.multisampled;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("specular-scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: attachment,
                depth_slice: None,
                resolve_target: Some(target),
                ops: wgpu::Operations {
                    // The grid overwrites every pixel, and only the resolved
                    // target is kept.
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            ..wgpu::RenderPassDescriptor::default()
        });
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_pipeline(&pipelines.grid);
        pass.draw(0..3, 0..1);

        let whole = Scissor {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        };
        let mut scissor = whole;
        for DrawOp {
            scissor: wanted,
            op,
        } in draws.iter()
        {
            let wanted = wanted.unwrap_or(whole);
            if wanted != scissor {
                pass.set_scissor_rect(wanted.x, wanted.y, wanted.width, wanted.height);
                scissor = wanted;
            }
            match op {
                Op::Quad { instance, texture } => {
                    let bind_group = match *texture {
                        QuadTexture::Page(page, layer) => self
                            .pages
                            .get(&page)
                            .and_then(|layers| layers.get(layer))
                            .map(|layer| &layer.bind_group),
                        QuadTexture::Image(image) => {
                            images.get(&image).map(|image| &image.bind_group)
                        }
                    };
                    let Some(bind_group) = bind_group else {
                        continue;
                    };
                    pass.set_pipeline(&pipelines.quad);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(1, bind_group, &[]);
                    pass.set_vertex_buffer(0, self.instance_buffer.slice());
                    pass.draw(0..4, *instance..*instance + 1);
                }
                Op::Shapes(range) => {
                    pass.set_pipeline(&pipelines.shape);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.shape_buffer.slice());
                    pass.draw(0..4, range.clone());
                }
                Op::Mesh(range) => {
                    pass.set_pipeline(&pipelines.mesh);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_vertex_buffer(0, mesh_vertices.slice());
                    pass.set_index_buffer(mesh_indices.slice(), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(range.clone(), 0, 0..1);
                }
                Op::Text { slot, space } => {
                    if let Some(text) = text {
                        text.render(&mut pass, *slot, *space);
                        // Held canvas text stretches the viewport.
                        pass.set_viewport(
                            0.0,
                            0.0,
                            size.width as f32,
                            size.height as f32,
                            0.0,
                            1.0,
                        );
                    }
                }
            }
        }
    }
}

/// The runs and columns of a text batch, in paint order.
fn text_draws<'a>(
    scene: &'a Scene,
    placed: &'a [super::place::Placed],
    batch: &'a Batch,
) -> impl Iterator<Item = TextDraw<'a>> {
    batch.members.iter().filter_map(|&at| {
        let placed = &placed[at];
        let item = &scene.items[placed.item];
        let text = match &item.draw {
            Draw::Text(run) => TextItem::Run(run),
            Draw::Column(column) => TextItem::Column(column),
            Draw::Page(_)
            | Draw::Rect(_)
            | Draw::Ellipse(_)
            | Draw::Polygon(_)
            | Draw::Path(_)
            | Draw::Image(_) => return None,
        };
        Some(TextDraw {
            text,
            clip: placed.clip,
            opacity: item.opacity,
        })
    })
}
